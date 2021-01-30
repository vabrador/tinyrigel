// tests/tests_windows.rs

use std::sync::mpsc::sync_channel;

use windows_bindings::windows::foundation::*;
use windows_bindings::windows::devices::enumeration::*;
use windows_bindings::windows::media::*;
use windows_bindings::windows::media::capture::*;
use windows_bindings::windows::media::capture::frames::*;
use windows_bindings::windows::media::media_properties::*;
use windows_bindings::windows::storage::streams::*;

use windows::Interface;

// Note on Vendor_ID and Product_ID as retrieved by device.Id on Windows
// ---
// As far sa I can tell, this ID is only ever accessible through Windows' specific negotiation pathway with the Rigel. macOS shows an entirely different set of data through uvc drivers, and Linux's uvc querying also reveals no such string -- not even the same data that macOS returns.
// Identifying Leap Motion devices may well be OS-specific, which is very counter-intuitive...

/// Vendor ID for Leap Motion. Leap Motion (now Ultraleap) camera devices contain this in their USB device ID string.
pub const VENDOR_ID__LEAP_MOTION: &'static str = "VID_2936";
/// Product ID for the Rigel, AKA the SIR 170. Rigel / SIR 170 devices contain this in their USB device ID string.
pub const PRODUCT_ID__RIGEL     : &'static str = "PID_1202";

#[test]
fn can_enumerate_video_devices() -> Result<(), &'static str> {
    println!("\n## can_enumerate_video_devices ##");

    // Request an enumeration of video devices from WinRT.
    let device_req = DeviceInformation::find_all_async_device_class(DeviceClass::VideoCapture);
    if device_req.is_err() { return Err("Failed to initiate device enumeration.") }
    let device_req = device_req.unwrap();

    // Spin in-place while we wait for the request to finish...
    naive_wait_for_async_op(&device_req, 1_000_000u64).expect("Wait for device");

    // Enumerate the DeviceInformation structs we get back from the request, noting if we find a Rigel.
    let mut device_idx = 0u32;
    let device_infos: DeviceInformationCollection = device_req.get_results().unwrap();
    for device_info in device_infos {
        println!("{}. Device Name: {}", device_idx, device_info.name().unwrap().to_string());

        if device_info.id().unwrap().to_string().contains(VENDOR_ID__LEAP_MOTION) {
            println!("\t- Device {} is an Ultraleap device.", device_idx);

            if device_info.id().unwrap().to_string().contains(PRODUCT_ID__RIGEL) {
                println!("\t- Device {} is a Rigel.", device_idx);
            }
        }

        device_idx += 1;
    }

    Ok(())
}

#[test]
fn can_retrieve_rigel_frame() -> windows::Result<()> {
    println!("\n## can_retrieve_rigel_frame ##");

    // We naively spin to wait out asynchronous requests. This sets the timeout in "loops" to wait for them (TODO: this SHOULD be a time duration!)
    let timeout = 20_000_000u64;

    // Get connected devices.
    let device_req = DeviceInformation::find_all_async_device_class(DeviceClass::VideoCapture)
        .expect("Failed to request VideoCapture devices");
    naive_wait_for_async_op(&device_req, timeout)
        .expect("Failed while waiting for device enumeration");

    // Find the connected Rigel (otherwise panic).
    let device_infos: DeviceInformationCollection = device_req.get_results()?;
    let rigel_device = device_infos.into_iter().find(|di| is_device_rigel(&di))
        .expect("Failed to find a connected Rigel (is your Rigel connected?)");

    // Initialize a MediaCapture object for the Rigel.
    let rigel_cap: MediaCapture = {

        let media_cap_init_settings = MediaCaptureInitializationSettings::new()?;
        
        // We're going to set the SourceGroup to the Rigel source group. Constructing the MediaCapture in this way will let us do MediaFrameReader stuff later.
        let rigel_src_group = MediaFrameSourceGroup::from_id_async(rigel_device.id()?)?;
        naive_wait_for_async_op(&rigel_src_group, timeout).expect("Failed waiting for MediaFrameSourceGroups");
        let rigel_src_group: MediaFrameSourceGroup = rigel_src_group.get_results()?;
        println!("Got frame source group for Rigel ID: display_name: {}, size: {}", rigel_src_group.display_name()?, rigel_src_group.source_infos()?.size()?);

        media_cap_init_settings.set_source_group(rigel_src_group)?;
        // media_cap_init_settings.set_video_device_id(rigel_device.id()?)?;

        // We'd like exclusive control of the device so we can possibly send commands to it to change camera parameters. (Not doing this right now, but maybe later?)
        media_cap_init_settings.set_sharing_mode(MediaCaptureSharingMode::ExclusiveControl)?;

        // We need to access frames from the CPU, so make that preference explicit.
        media_cap_init_settings.set_memory_preference(MediaCaptureMemoryPreference::Cpu)?;

        // The device doesn't provide audio -- and we don't need it anyway.
        media_cap_init_settings.set_streaming_capture_mode(StreamingCaptureMode::Video)?;

        // Initialize the MediaCapture object.
        let media_cap = capture::MediaCapture::new()
            .expect("Failed to create MediaCapture object");
        let init_req = media_cap.initialize_with_settings_async(media_cap_init_settings)?;
        
        naive_wait_for_async_act(&init_req, timeout).unwrap();

        media_cap
    };

    // The catch: The default resolution reported for the Rigel (640x480) won't work. We can pick any other resolution that the device reports as supported, though, and it will work fine.
    //
    // For our purposes here, we'll use 384x384 @ 90fps.

    // Get available MediaEncodingProperties for streaming video from the device.
    let avail_media_stream_props = rigel_cap.video_device_controller()
        .expect("Failed to get VideoDeviceController for Rigel capture")
        .get_available_media_stream_properties(capture::MediaStreamType::VideoRecord)
        .expect("Failed to get available MediaStreamProperties for VideoRecord.");

    // Find the 384x384 VideoEncodingProperties we're looking for.
    let vep_384x384_90fps = avail_media_stream_props.into_iter()
        // Only retrieve VideoEncodingProperties, and cast to that type.
        .filter_map(|mep| {
            if mep.r#type().unwrap().to_string() == "Video" {
                let vep: VideoEncodingProperties = mep.cast().unwrap();
                println!("Supported VideoEncodingProperties: Width: {}, Height: {}, Framerate: {}", vep.width().unwrap(), vep.height().unwrap(), {
                    let frame_rate = vep.frame_rate().unwrap();
                    frame_rate.numerator().unwrap() / frame_rate.denominator().unwrap()
                });
                Some(vep)
            } else { None }
        })
        // Specifically find the 384x384 encoding properties.
        .find(|vep|
            vep.width().unwrap() == 384 &&
            vep.height().unwrap() == 384 &&
            media_ratio_to_value(&vep.frame_rate().unwrap()) == 90
        )
        .expect("Failed to find supported encoding for 384x384 @ 90fps");
    
    // Set the Rigel to 384x384 @ 90fps.
    naive_wait_for_async_act(
        &rigel_cap.video_device_controller()?.set_media_stream_properties_async(MediaStreamType::VideoRecord, vep_384x384_90fps)
            .expect("Failed to set Rigel stream to 384x384"),
        timeout
    )
        .expect("Failed waiting while setting Rigel stream to 384x384");

    // MediaFrameSource for threaded frame acquisition //
    // ---

    let rigel_media_frame_src: MediaFrameSource = rigel_cap.frame_sources()?.first()?.current()?.value()?;
    println!("Got media frame source... Current format: Framerate: {:?}", media_ratio_to_value(&rigel_media_frame_src.current_format()?.frame_rate()?));
    
    let rigel_media_frame_reader = rigel_cap.create_frame_reader_async(rigel_media_frame_src)?;
    let rigel_media_frame_reader = naive_wait_and_get_op_results(&rigel_media_frame_reader, timeout)?;

    // We always only care about the most recent frame; if frame processing takes too long, further incoming frames will simply be dropped. (We don't expect this to happen though!)
    rigel_media_frame_reader.set_acquisition_mode(MediaFrameReaderAcquisitionMode::Realtime)?;
    
    // Quick synchronization pair -- the transmitter can send from many threads.
    let (tx_write_img, rx_write_img) = sync_channel::<u32>(0);
    let (tx_finish   , rx_finish   ) = sync_channel::<u32>(0);

    let evt_token = rigel_media_frame_reader.frame_arrived(TypedEventHandler::new(move |reader: &Option<MediaFrameReader>, _args: &Option<MediaFrameArrivedEventArgs>| {
        // Convert &Option<T> to Option<&T> and unwrap the Option. (This would panic if "reader" is 'null'.)
        let reader = reader.as_ref().unwrap();

        // According to the documentation for TryAcquireLatestFrame, it may return null. Presumably this would return an Err() from try_acquire_latest_frame... but I'm not actually sure if that's true in the Rust bindings or if it instead returns a hidden "null" behind the MediaFrameReference handle that is only apparent when accessing some property through that handle. So we check both possibilities here.
        let latest_frame = match reader.try_acquire_latest_frame() {
            Ok(latest_frame) => { Some(latest_frame) },
            Err(e) => { println!("Error acquiring frame: {}", e.message()); None }
        };
        if latest_frame.is_none() {
            println!("No latest frame available. Ending this frame handler invocation early.");
            return Ok(());
        }
        let latest_frame = latest_frame.unwrap();
        // Now we have a MediaFrameReference, but we still can't necessarily be sure if the handle is non-null. We'll check that here by seeing if we can retrieve the frame's format.
        match latest_frame.format() {
            Ok(_) => {},
            Err(e) => {
                println!("Error retrieving frame format: {}", e.message());
                return Ok(())
            }
        };
        println!("Got frame! Format is: {:?}", latest_frame.format()?.subtype()?);

        // By now, we know we have a valid frame.
        let bitmap = latest_frame.video_media_frame()?.software_bitmap()?;
        println!("Got bitmap! Pixel format: {}, {}x{}", bitmap.bitmap_pixel_format()?.0, bitmap.pixel_width()?, bitmap.pixel_height()?);

        // Leap Motion devices pretend to be giving frames in YUY2 format (which uses 4 bytes to encode 2 pixels at a time), but it's really 8-bit grayscale, with each data row containing the left-image row followed by the right-image row.

        const FRAME_YUY2_BYTES_PER_PIXEL: usize = 2;
        const FRAME_PX_COUNT: usize = 384 * 384; // 147,456
        const FRAME_NUM_BYTES: usize = FRAME_PX_COUNT * FRAME_YUY2_BYTES_PER_PIXEL;
        let frame_buffer = Buffer::create(FRAME_NUM_BYTES as u32)?;
        // Even though we're calling "clone()", we're actually cloning the WinRT *handle* for the buffer, so "frame_buffer" will still wind up containing copied bitmap data.
        bitmap.copy_to_buffer(frame_buffer.clone())?;
        let frame_data_reader = DataReader::from_buffer(frame_buffer)?;

        // The frame is too large for the stack, so we initialize a Vec, which stores its data on the heap. Note we can't do Box::new([0u8; NUM_BYTES]) because the argument would first be constructed on the stack and then moved to the heap.
        let mut heap_data_vec = Vec::with_capacity(FRAME_NUM_BYTES);
        heap_data_vec.resize(FRAME_NUM_BYTES, 0u8);
        // Once we have the vec, we can get a mutable reference to the array (or 'slice,' which is a pointer + length struct), and use it normally.
        let heap_data_slice = heap_data_vec.as_mut_slice();
        let mut_heap_arr_ref = heap_data_slice.as_mut();

        println!("FRAME: Available data length: {}", frame_data_reader.unconsumed_buffer_length()?);
        println!("FRAME: Buffer length: {}", mut_heap_arr_ref.len());
        frame_data_reader.read_bytes(mut_heap_arr_ref)
            .expect("Failed to read frame data");

        // We have the image, now get permission from the main thread to write to disk.
        // This blocks for the receiving end of the channel -- the main thread.
        let ok_to_write = match tx_write_img.send(0) {
            Ok(_) => {
                println!("[FrameArrived] Got OK-to-write-to-disk signal.");
                true
            },
            Err(_) => {
                println!("[FrameArrived] Failed to get signal for writing to disk."); false
            }
        };
        // If it's not OK to write, just return.
        if !ok_to_write { return Ok(()); }

        // Save the image as a PNG as a test.
        use image::GenericImageView;
        let mut img = image::DynamicImage::new_luma8(384 * 2, 384);
        let img_luma8 = img.as_mut_luma8().unwrap();
        img_luma8.copy_from_slice(mut_heap_arr_ref);
        println!("Copied image from frame data, {}x{}", img.width(), img.height());
        img.save("test.png").unwrap();
        println!("Invoked write to test.png");
        
        // Transmit because we're done. This blocks and waits for the receiver.
        match tx_finish.send(0) {
            Ok(_) => println!("[FrameArrived] Sent finished signal."),
            Err(_) => println!("[FrameArrived] Failed to send finished signal, channel probably hung up on the other end.")
        };

        Ok(())
    }))?;
    println!("Frame arrived event subscribed. EventRegistrationToken: {:?}", evt_token.value);

    let start_status = rigel_media_frame_reader.start_async()?;
    let start_status = naive_wait_and_get_op_results(&start_status, timeout)?;

    println!("Start status was: {} (0 is success)", start_status.0);

    // Receive the signal from the frame thread that a frame is ready. The frame thread waits for this signal before writing to disk.
    rx_write_img.recv().unwrap();
    
    // After the OK-to-write signal, wait for the signal from the frame thread that the write is done.
    let _sig_int = rx_finish.recv().unwrap();
    println!("Got finished signal from a frame thread");

    // Stop the capture and clean up.
    //
    // The frame thread's handler may be invoked again before wrap-up has had a chance to occur here. Since we don't want to interrupt a possible disk write occurring in the handler on the frame thread, the handler's disk write is gated by the OK-to-write signal we send from the main thread. We only provide this signal once, just above; so it's OK to interrupt the frame thread at any time after we've received the "frame finished" signal as long as we don't invoke rx_write_img.recv() again before the program terminates.
    let stop_action = rigel_media_frame_reader.stop_async()?;
    naive_wait_for_async_act(&stop_action, timeout).unwrap();
    rigel_cap.close()?;
    
    Ok(())
}

// Helper Methods - Devices //

/// Checks the DeviceInformation ID string to see if it contains the Leap Motion vendor ID and Rigel (aka SIR 170) product ID. If it does, returns true, otherwise returns false.
fn is_device_rigel(device_info: &DeviceInformation) -> bool {
    let device_id_str = device_info.id().map(|hstr| hstr.to_string()).unwrap_or_default();
    
    device_id_str.contains(VENDOR_ID__LEAP_MOTION) &&
    device_id_str.contains(PRODUCT_ID__RIGEL)
}

fn media_ratio_to_value(media_ratio: &MediaRatio) -> u32 {
    return media_ratio.numerator().unwrap() / media_ratio.denominator().unwrap();
}

// Helper Methods - General WinRT //

fn naive_wait_and_get_op_results<T: windows::RuntimeType>(async_op: &IAsyncOperation<T>, loop_limit: u64) -> ::windows::Result<T> {
    naive_wait_for_async_op(async_op, loop_limit).unwrap();
    async_op.get_results()
}

/// Spin in place waiting for the async operation to complete. Not very graceful.
///
/// TODO: Use a time-based timeout and not a CPU-clock-based timeout.
// fn naive_wait_for_device<T>(device_req: &IAsyncOperation<T>, loop_limit: u64) -> Result<(), &'static str> {
fn naive_wait_for_async_op<T: windows::RuntimeType>(async_op: &IAsyncOperation<T>, loop_limit: u64) -> Result<(), &'static str> {
    let mut loops = 0u64;
    loop {
        if async_op.status().unwrap() == AsyncStatus::Completed {
            break;
        }

        loops += 1; if loops >= loop_limit { break; }
    }
    if loops >= loop_limit { return Err("Timeout while waiting for the IAsyncOperation to complete."); }

    Ok(())
}

/// Spin in place waiting for the async action to complete. Not very graceful.
///
/// TODO: Use a time-based timeout and not a CPU-clock-based timeout.
// fn naive_wait_for_device<T>(device_req: &IAsyncOperation<T>, loop_limit: u64) -> Result<(), &'static str> {
fn naive_wait_for_async_act(req: &IAsyncAction, loop_limit: u64) -> Result<(), &'static str> {
    let mut loops = 0u64;
    loop {
        if req.status().unwrap() == AsyncStatus::Completed {
            break;
        }

        loops += 1; if loops >= loop_limit { break; }
    }
    if loops >= loop_limit { return Err("Timeout while waiting for the IAsyncAction to complete.")}

    Ok(())
}
