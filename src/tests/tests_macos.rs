// tests/tests_macos.rs
//
// This file may look funny, as using the objc crate to cross-communicate with Objective-C involves heavy use of the msg_send![] macro. This macro allows interoperable calls to Objective-C methods on Objective-C objects, and mimics the syntax of such Objective-C method calls.
//
// Such calls don't have many safety guarantees and type annotations need to be provided as hints to the macro to correctly link to the appropriate methods. As such, this code is more brittle and unsafe than standard (safe) Rust.
//
// Check out the objc crate for more info.

use std::{ffi::{CStr}, ptr, sync::{Mutex, mpsc::{Receiver, Sender, channel}}, time::Duration};
use cocoa_foundation::{base::id, base::{BOOL, NO, YES, nil}, foundation::{NSInteger, NSString}};
use core_foundation::{base::TCFType, error::{CFError, CFErrorRef}};
use objc::{class, declare::ClassDecl, msg_send, rc::StrongPtr, runtime::{Object, Sel}, sel, sel_impl};
use ptr::null;
use lazy_static::lazy_static;
// use objc::runtime::Object; <-- id, provided by cocoa_foundation, is short-hand for *mut objc::runtime::Object.

// These tests are only valid on macOS and iOS.
#[cfg(any(target_os = "macos", target_os = "ios"))]

// Framework dependencies.
// ---
// Linking frameworks in this way additionally requires a build step.

// AVFoundation. AVFoundation exposes classes and class functions in Objective-C, so we interact with AVFoundation-related invocations using the `objc` crate.
#[link(name = "AVFoundation", kind = "framework")]

// CoreMedia, with plain C functions. Here any "plain functions" we seek to invoke need to be declared and any structs they return we need to define as for plain C ffi.
// 
// The core_foundation shows some examples for how to link in plain C functions from Core frameworks. For example:
// https://github.com/servo/core-foundation-rs/blob/master/core-graphics/src/event_source.rs
#[link(name = "CoreMedia", kind = "framework")]

// CoreVideo, also with plain C functions. Functions provided by CoreVideo are used to query frame data in the camera frame callback delegate.
#[link(name = "CoreVideo", kind = "framework")]

// Linked method declarations.
extern "C" {
    /// `CMVideoDimensions CMVideoFormatDescriptionGetDimensions(CMVideoFormatDescriptionRef videoDesc);`
    ///
    /// See: https://developer.apple.com/documentation/coremedia/1489287-cmvideoformatdescriptiongetdimen?language=objc
    fn CMVideoFormatDescriptionGetDimensions(videoDesc: id) -> CMVideoDimensions;

    /// `CVImageBufferRef CMSampleBufferGetImageBuffer(CMSampleBufferRef sbuf);`
    ///
    /// See: https://developer.apple.com/documentation/coremedia/1489236-cmsamplebuffergetimagebuffer?language=objc
    fn CMSampleBufferGetImageBuffer(sbuf: *const Object) -> id;

    /// `CVReturn CVPixelBufferLockBaseAddress(CVPixelBufferRef pixelBuffer, CVPixelBufferLockFlags lockFlags);`
    /// For lockFlags readOnly, pass 0x00000001.
    ///
    /// See: https://developer.apple.com/documentation/corevideo/1457128-cvpixelbufferlockbaseaddress?language=objc
    fn CVPixelBufferLockBaseAddress(pixelBuffer: id, lockFlags: u64) -> i32;

    /// `CVReturn CVPixelBufferUnlockBaseAddress(CVPixelBufferRef pixelBuffer, CVPixelBufferLockFlags unlockFlags);`
    /// If specific lockFlags were passed with LockBaseAddress, the same flags must be passed with Unlock.
    ///
    /// See: https://developer.apple.com/documentation/corevideo/1456843-cvpixelbufferunlockbaseaddress?language=objc
    fn CVPixelBufferUnlockBaseAddress(pixelBuffer: id, lockFlags: u64) -> i32;

    /// `size_t CVPixelBufferGetBytesPerRow(CVPixelBufferRef pixelBuffer);`
    ///
    /// See: https://developer.apple.com/documentation/corevideo/1456964-cvpixelbuffergetbytesperrow?language=objc
    fn CVPixelBufferGetBytesPerRow(pixelBuffer: id) -> libc::size_t;

    /// `size_t CVPixelBufferGetWidth(CVPixelBufferRef pixelBuffer);`
    ///
    /// See: https://developer.apple.com/documentation/corevideo/1457241-cvpixelbuffergetwidth?language=objc
    fn CVPixelBufferGetWidth(pixelBuffer: id) -> libc::size_t;

    /// `size_t CVPixelBufferGetHeight(CVPixelBufferRef pixelBuffer);`
    ///
    /// See: https://developer.apple.com/documentation/corevideo/1456666-cvpixelbuffergetheight?language=objc
    fn CVPixelBufferGetHeight(pixelBuffer: id) -> libc::size_t;

    /// `void * CVPixelBufferGetBaseAddress(CVPixelBufferRef pixelBuffer);`
    ///
    /// See: https://developer.apple.com/documentation/corevideo/1457115-cvpixelbuffergetbaseaddress
    fn CVPixelBufferGetBaseAddress(pixelBuffer: id) -> *const libc::c_void;

    /// `dispatch_queue_t dispatch_queue_create(const char *label, dispatch_queue_attr_t attr);`
    ///
    /// See: https://developer.apple.com/documentation/dispatch/1453030-dispatch_queue_create
    fn dispatch_queue_create(label: *const libc::c_char, attr: id) -> id;

    /// `void dispatch_release(dispatch_object_t object);`
    ///
    /// See: https://developer.apple.com/documentation/dispatch/1496328-dispatch_release
    fn dispatch_release(object: id);
}

// C-interop struct definitions.
#[repr(C)]
pub struct CMVideoDimensions {
    pub width: i32,
    pub height: i32,
}

// Tests
// ---

// TODO: Add "backends/macos-xcode-ref"

lazy_static! {
    /// Receiver for the frame thread to receive an OK-to-write signal from the main thread to write the retrieved image to disk.
    static ref FRAME_OK_TO_WRITE_RX: Mutex<Option<Receiver<i32>>> = Mutex::new(None);
    /// Sender for the frame thread to send a done-writing signal to the main thread so the main thread can safely stop the frame thread and exit.
    static ref FRAME_DONE_WRITING_TX: Mutex<Option<Sender<i32>>> = Mutex::new(None);
}

#[test]
fn can_retrieve_rigel_frame() -> Result<(), &'static str> {
    println!("=== can_retrieve_rigel_frame ===");

    // Attempt to enumerate devices and retrieve a connected Rigel.
    let rigel_device: Option<id> = unsafe {
        // NSArray<AVCaptureDevice *> *
        // TODO: Change "devices" call to instead use AVCaptureDeviceDiscoverySession.
        // See: https://developer.apple.com/documentation/avfoundation/avcapturedevicediscoverysession?language=objc
        let devices: id = msg_send![class!(AVCaptureDevice), devices];
        let devices_count: NSInteger = msg_send![devices, count];
        println!("AVCaptureDevice.devices devices_count is {}", devices_count);

        let mut rigel: Option<id> = None;
        for device_idx in 0..devices_count {
            let device: id = msg_send![devices, objectAtIndex: device_idx];
            let model_id: id = msg_send![device, modelID];
            let model_id_str = CStr::from_ptr(model_id.UTF8String()).to_string_lossy().to_string();
            println!("Device model ID... {}", &model_id_str);

            if device_model_id_is_rigel(&model_id_str) {
                println!("Found Rigel. Model id was: {}", model_id_str);
                rigel = Some(device);
                break;
            }
        }

        rigel
    };
    if rigel_device.is_none() {
        return Err("Failed to find connected Rigel. Is your Rigel connected?");
    }
    let rigel_device = rigel_device.unwrap();

    // To check whether this reference is still valid after the unsafe block, let's try to print its description.
    println!("Using connected Rigel: {}", get_objc_object_desc(rigel_device));

    // Once we have a Rigel, we need to find the correct format to use.
    //
    // On Windows but possibly not on other systems, the default format reported by the Rigel is not actually valid, and will prevent the Rigel from capturing frames.
    //
    // Regardless, in this case, we're interested in 384x384 capture @ 90 fps.
    let format_384x384_90fps: Option<id> = unsafe {
        // NSArray<AVCaptureDeviceFormat *> *
        let formats: id = msg_send![rigel_device, formats];
        let formats_count: NSInteger = msg_send![formats, count];

        let mut format_384x384_90fps: id = nil;
        for format_idx in 0..formats_count {
            let format: id = msg_send![formats, objectAtIndex: format_idx];

            // CMFormatDescriptionRef
            let format_desc: id = msg_send![format, formatDescription];
            let dimensions = CMVideoFormatDescriptionGetDimensions(format_desc);
            println!("Detected available format dimensions: {}x{}", dimensions.width, dimensions.height);
            if dimensions.width == 384 && dimensions.height == 384 {
                format_384x384_90fps = format;
            }
        }

        if format_384x384_90fps != nil {
            Some(format_384x384_90fps)
        } else {
            None
        }
    };
    if format_384x384_90fps.is_none() {
        return Err("Failed to find a 384x384 resolution for the Rigel. Exiting.");
    }
    let format_384x384_90fps = format_384x384_90fps.unwrap();

    // Lock the Rigel configuration and set its format. We'll unlock after invoking startRunning, per Apple docs.
    // https://developer.apple.com/documentation/avfoundation/avcapturedevice/1387810-lockforconfiguration
    let did_configure_rigel_format = unsafe {
        // lockForConfiguration is a subtle example of Rust <-> Core Foundation interop where we want to pass a CFError double-pointer for the locking function to provide us a possible error message.
        //
        // An example for working with Core Foundation "nullable pointer to nullable error" situations like this can be found here:
        // https://github.com/servo/core-foundation-rs/blob/master/core-foundation/src/propertylist.rs
        // ..or, the same reference but as a permalink to the commit:
        // https://github.com/servo/core-foundation-rs/blob/faf0d7c0ffe902b434cb27c3fe15f160cc1d40a8/core-foundation/src/propertylist.rs
        let mut config_err: CFErrorRef = ptr::null_mut();

        println!("Attempting to lock rigel_device for configuration.");
        let did_lock_succeed: BOOL = msg_send![rigel_device, lockForConfiguration: &mut config_err];

        if did_lock_succeed == 0 {
            let err: CFError = TCFType::wrap_under_create_rule(config_err);
            println!("Failed to lock rigel_device for configuration. The returned NSError description was: {}\nMost likely, the Rigel is in use by another process, so this process can't take exclusive control of the device.", err.description());
            false
        } else {
            println!("Attempting to set rigel active format...");
            let _: () = msg_send![rigel_device, setActiveFormat: format_384x384_90fps];
            true
        }
    };
    if !did_configure_rigel_format {
        return Err("Failed to configure Rigel to 384x384 @ 90fps format.");
    } else {
        println!("Set Rigel to 384x384 @ 90fps format.");
    }

    // Initialize the Rigel capture device input node.
    let rigel_input: Option<id> = unsafe {
        let mut init_err: CFErrorRef = ptr::null_mut();

        let rigel_input: id = msg_send![class!(AVCaptureDeviceInput), deviceInputWithDevice: rigel_device error: &mut init_err];

        if rigel_input == nil {
            let err: CFError = TCFType::wrap_under_create_rule(init_err);
            println!("Error initializing AVCaptureDeviceInput for Rigel: {}", err.description());

            None
        } else {
            Some(rigel_input)
        }
    };
    if rigel_input.is_none() {
        return Err("Failed to initialize AVCaptureDeviceInput for Rigel.");
    }
    println!("Initialized AVCaptureDeviceInput for Rigel.");
    let rigel_input = rigel_input.unwrap();

    // Initialize the capture session and add the input node to it.
    let capture_session: StrongPtr = unsafe {
        let allocated_capture_session: id = msg_send![class!(AVCaptureSession), alloc];
        if allocated_capture_session == nil { return Err("Failed to allocate a new AVCaptureSession."); }
        let capture_session: id = msg_send![allocated_capture_session, init];

        StrongPtr::new(capture_session)
    };
    println!("Trying AVCaptureSession canAddInput");
    let can_add_rigel_input: BOOL = unsafe { msg_send![*capture_session, canAddInput: rigel_input] };
    if can_add_rigel_input == NO {
        return Err("Unable to add the rigel_input node to the new capture_session.");
    }
    println!("Trying AVCaptureSession addInput");
    unsafe { let _: () = msg_send![*capture_session, addInput: rigel_input]; }
    println!("Added the rigel_input node to the new capture_session.");

    // Initialize the frame callback output node with "YUY2" format. (The data we'll get is only pretending to be YUY2, but that's OK!)
    //
    // First, declare the delegate class.
    //
    // Currently using as a reference:
    // https://github.com/ndarilek/tts-rs/blob/d3e05b5a7a642eb3212528ecc8cdedd406673213/src/backends/av_foundation.rs
    let capture_output: StrongPtr = unsafe {
        let allocated_capture_output: id = msg_send![class!(AVCaptureVideoDataOutput), alloc];
        if allocated_capture_output == nil { return Err("Failed to allocate a new AVCaptureVideoDataOutput."); }
        let capture_output: id = msg_send![allocated_capture_output, init];

        StrongPtr::new(capture_output)
    };
    // We have to provide a dispatch queue to the output capture node.
    let dispatch_queue = unsafe { dispatch_queue_create(null(), nil /* DISPATCH_QUEUE_SERIAL -- to guarantee order. Xcode reveals this is actually defined to be NULL, so let's hope that never changes! */) };
    // We also specify that we just want to discard late frames.
    unsafe { let _: () = msg_send![*capture_output, setAlwaysDiscardsLateVideoFrames: YES]; }

    // Declare and implement the frame callback delegate class.
    let mut frame_delegate_decl = ClassDecl::new("TinyRigelAVCapture", class!(NSObject)).unwrap();
    
    // AVCaptureVideoDataOutputSampleBufferDelegate
    // ---
    // Implementation for the frame callback.
    //
    // Selector: sel!(captureOutput:didOutputSampleBuffer:fromConnection:)
    extern "C" fn capture_output_did_output_sample_buffer_from_connection(
        _this: &Object,
        _: Sel,
        _output: id, /* AVCaptureOutput * */
        sample_buffer: *const Object, /* immutable CMSampleBufferRef */
        _connection: id /* AVCaptureConnection * */
    ) {
        println!("[Frame] Got frame callback.");

        // Wait for the ok-to-write signal.
        // https://doc.rust-lang.org/nightly/std/sync/mpsc/index.html
        //
        // We're waiting BEFORE locking the pixel buffer for copying, so that if the image frame thread shuts down, we don't leave the pixel buffer's address locked.
        {
            let ok_to_write_rx_lock = FRAME_OK_TO_WRITE_RX.try_lock();
            if ok_to_write_rx_lock.is_err() {
                println!("[Frame] Failed to acquire ok_to_write. Aborting this frame callback. Error was: {}", ok_to_write_rx_lock.err().unwrap());
                return;
            }
            let ok_to_write_rx = ok_to_write_rx_lock.unwrap();
            if ok_to_write_rx.is_none() {
                println!("[Frame] OK-to-write receiver was None (did the main thread neglect to set it before launching the capture session?). Aborting this frame callback.");
                return;
            }
            println!("[Frame] Waiting for ok-to-write...");
            let res = ok_to_write_rx.as_ref().unwrap().recv_timeout(Duration::from_millis(500));
            if res.is_err() {
                println!("[Frame] Receive timed out or the channel hung up. Aborting frame callback.");
                return;
            }
            println!("[Frame] Received OK-to-write.");
        };

        // Safety: "The caller does not own the returned buffer, and must retain it explicitly if the caller needs to maintain a reference to it." (We do not need to maintain a reference to it.)
        let img_buf: id = unsafe { CMSampleBufferGetImageBuffer(sample_buffer) };
        if img_buf == nil {
            return;
        }
        
        // CVPixelBufferLockBaseAddress(imageBuffer, kCVPixelBufferLock_ReadOnly);
        unsafe {
            let cv_return = CVPixelBufferLockBaseAddress(img_buf, 1u64);
            if cv_return != 0 {
                println!("Non-zero return from CVPixelBufferLockBaseAddress: {}", cv_return);
                return;
            }
        }
        
        let (bytes_per_row, _width, height) = unsafe {
            let bytes_per_row = CVPixelBufferGetBytesPerRow(img_buf);
            let width: libc::size_t = CVPixelBufferGetWidth(img_buf);
            let height = CVPixelBufferGetHeight(img_buf);
            (bytes_per_row, width, height)
        };

        // Copy to a safe Vec<u8>.
        let copied_frame_data = unsafe {
            let src_buf = CVPixelBufferGetBaseAddress(img_buf) as *const u8;
            let src_buf_as_slice = std::slice::from_raw_parts(src_buf, bytes_per_row * height);

            println!("[Frame] src_buf_as_slice is {} bytes in length.", src_buf_as_slice.len());

            const FRAME_YUY2_BYTES_PER_PIXEL: usize = 2;
            const FRAME_PX_COUNT: usize = 384 * 384; // 147,456
            const FRAME_NUM_BYTES: usize = FRAME_PX_COUNT * FRAME_YUY2_BYTES_PER_PIXEL;

            println!("[Frame] We expect there to be {} bytes per frame.", FRAME_NUM_BYTES);

            if src_buf_as_slice.len() != FRAME_NUM_BYTES {
                println!("[Frame] Unexpected length mismatch, src_buf {} bytes != FRAME_NUM_BYTES of {} bytes.", src_buf_as_slice.len(), FRAME_NUM_BYTES);
                return;
            }
            
            let mut heap_dst_vec = Vec::with_capacity(FRAME_NUM_BYTES);
            heap_dst_vec.resize(FRAME_NUM_BYTES, 0u8);
            let heap_dst_slice = heap_dst_vec.as_mut_slice();
            heap_dst_slice.copy_from_slice(src_buf_as_slice);

            heap_dst_vec
        };
        
        // Unlock the pixel buffer before going further.
        // CVPixelBufferUnlockBaseAddress(imageBuffer, 0);
        unsafe {
            let cv_return = CVPixelBufferUnlockBaseAddress(img_buf, 1u64);
            if cv_return != 0 {
                println!("[Frame] Non-zero return from CVPixelBufferUnlockBaseAddress: {}", cv_return);
                return;
            }
        }
        
        // Shouldn't need any unsafe or interop code from here on.

        // Save the image as a PNG as a test.
        use image::GenericImageView;
        let mut img = image::DynamicImage::new_luma8(384 * 2, 384);
        let img_luma8 = img.as_mut_luma8().unwrap();
        img_luma8.copy_from_slice(copied_frame_data.as_slice());
        println!("[Frame] Copied image from frame data, {}x{}", img.width(), img.height());
        img.save("test.png").unwrap();
        println!("[Frame] Invoked write to test image");

        // Transmit a "done" signal.
        {
            let done_writing_tx_lock = FRAME_DONE_WRITING_TX.try_lock();
            if done_writing_tx_lock.is_err() {
                println!("[Frame] Failed to acquire done_writing_tx_lock. Aborting this frame callback. Error was: {}", done_writing_tx_lock.err().unwrap());
                return;
            }
            let done_writing_tx = done_writing_tx_lock.unwrap();
            if done_writing_tx.is_none() {
                println!("[Frame] Done-writing transmitter was None (did the main thread neglect to set it before launching the capture session?). Aborting this frame callback.");
                return;
            }
            let res = done_writing_tx.as_ref().unwrap().send(0i32);
            if res.is_err() {
                println!("Err sending done signal: {}", res.err().unwrap());
            }
            println!("[Frame] Frame sent done-writing signal.");
        };
        // done_writing_tx.

        println!("[Frame] Frame callback done.");
    }

    unsafe {
        // Add the frame callback method to the delegate class declaration.
        frame_delegate_decl.add_method(
            sel!(captureOutput:didOutputSampleBuffer:fromConnection:),
            capture_output_did_output_sample_buffer_from_connection
                as extern "C" fn(&Object, Sel, id, *const Object, id) -> (),
        );
    }

    let frame_delegate_cls = frame_delegate_decl.register();
    // TinyRigelAVCapture *
    let frame_delegate_obj: StrongPtr = unsafe {
        let instance: id = msg_send![frame_delegate_cls, new];
        StrongPtr::new(instance)
    };
    unsafe {
        let _: () = msg_send![*capture_output, setSampleBufferDelegate: *frame_delegate_obj queue: dispatch_queue];
    }

    // Skipped for now: Attempting to set the capture_output video_settings to a specific format.
    // ---
    // Original Obj-C reference:
    //
    // // kCVPixelFormatType_422YpCbCr8 == 846624121
    // id formatTypeNum = [NSNumber numberWithInt: kCVPixelFormatType_422YpCbCr8];
    // // kCVPixelFormatType_422YpCbCr8_yuvs == 2037741171
    // // id formatTypeNum = [NSNumber numberWithInt: kCVPixelFormatType_422YpCbCr8_yuvs];
    // // kCVPixelBufferPixelFormatTypeKey => @"PixelFormatType" (NSString)
    // id formatTypeKey = (id)kCVPixelBufferPixelFormatTypeKey;
    // NSLog(@"kCVPixelBufferPixelFormatTypeKey: %@", kCVPixelBufferPixelFormatTypeKey);
    // id setVideoSettingsDict = [NSDictionary dictionaryWithObject: formatTypeNum forKey: @"PixelFormatType"];
    // [captureOutput setVideoSettings: setVideoSettingsDict];
    // NSLog(@"Set captureOutput videoSettings pixel format to %u, aka '2vuy', via kCVPixelFormatType_422YpCbCr8.", kCVPixelFormatType_422YpCbCr8);

    // Add the output node, with the frame callback delegate, to the capture session.
    let can_add_capture_output: BOOL = unsafe { msg_send![*capture_session, canAddOutput: *capture_output] };
    if can_add_capture_output == NO {
        return Err("Unable to add capture_output to capture_session.");
    }
    unsafe { let _: () = msg_send![*capture_session, addOutput: *capture_output]; }
    println!("Added output with frame callback delegate to capture_session.");

    // Before starting the capture session, set the global receiver and transmitters for the frame callback thread to access. It would be much more graceful if we could just move the relevant channel accessors into a closure, but because we're instead passing a function handle and it would be difficult to give the Objective-C delegate class access to the Rust sync channels, we're just using lazy_static Mutex-wrapped channel accessors.
    let (frame_ok_to_write_tx_main, frame_ok_to_write_rx) = channel::<i32>();
    *FRAME_OK_TO_WRITE_RX.lock().expect("Failed to acquire frame OK-to-write receiver mutex lock.") = Some(frame_ok_to_write_rx);
    let (frame_done_writing_tx, frame_done_writing_rx_main) = channel::<i32>();
    *FRAME_DONE_WRITING_TX.lock().expect("Failed to acquire frame done-writing transmitter mutex lock.") = Some(frame_done_writing_tx);

    // Start the capture session and unlock the device configuration.
    unsafe {
        let _: () = msg_send![*capture_session, startRunning];
        let _: () = msg_send![rigel_device, unlockForConfiguration];
    }
    println!("Unlocked the Rigel configuration.");

    // Confirm whether the capture_session is running.
    let is_running: BOOL = unsafe { msg_send![*capture_session, isRunning] };
    if is_running == NO {
        return Err("Capture session failed to be running after invoking startRunning.");
    }
    println!("Capture session is running...");

    // Wait a bit before transmitting OK-to-write to test if the frame stabilizes if the cameras are left on for a moment.
    println!("Waiting for a moment.");
    let mut x = 0u64;
    for _ in 0..20_000_000u64 {
        x += 1;
    }
    println!("{}", x);
    
    // Transmit the "ok to write" signal.
    let res = frame_ok_to_write_tx_main.send(0);
    if res.is_err() {
        println!("Failed to transmit OK-to-write signal to frame thread: {}", res.err().unwrap());
        return Err("Failed to transmit OK-to-write signal to frame thread.");
    }
    println!("Sent OK-to-write signal.");

    // Wait for the "done writing" signal.
    println!("Waiting for done-writing...");
    let res = frame_done_writing_rx_main.recv_timeout(Duration::from_millis(4000));
    if res.is_err() {
        println!("Error waiting for done-writing from frame thread: {}", res.err().unwrap());
        return Err("Error waiting for done-writing");
    }
    println!("Got done-writing.");

    // Hang up on the frame thread so it aborts immediately in the future.
    drop(frame_ok_to_write_tx_main);
    drop(frame_done_writing_rx_main);

    // Stop...
    unsafe { let _: () = msg_send![*capture_session, stopRunning]; }
    println!("Invoked stopRunning on capture_session.");
    // Frame is now done writing to disk.

    // Clean up.
    unsafe { let _: () = msg_send![*capture_session, removeInput: rigel_input]; }
    unsafe { let _: () = msg_send![*capture_session, removeOutput: *capture_output]; }

    // Memory safety:
    // ---
    // We use StrongPtrs to wrap allocated objects so that they are released when the StrongPtr wrapper is dropped.
    // See: https://github.com/SSheldon/rust-objc#reference-counting
    //
    // For this reason, we don't manually invoke "release" messages on any of the objects we own.
    
    // TODO: Run a memory leak test on this. I don't trust this code at aaaaalllllllll.

    // One exception to the above StrongPtr tracking is the dispatch_queue. The docs direct us to invoke the dispatch_release method and pass it the dispatch_queue when we are finished with it.
    // TODO: Is it possible to just rely on a release message for this too?
    unsafe { dispatch_release(dispatch_queue); }

    println!("Done.");
    Ok(())
}

/// Returns whether the model_id string contains the UVC Vendor ID string corresponding to "LEAP Motion" and the product ID corresponding to the Rigel.
///
/// For some reason, this pair of IDs differs from the Vendor ID and Product ID reported in the `.Id` string when accessing a Rigel through the default Windows USB video device driver. I don't know why. It's possible the Rigel firmware is reporting UVC device information to macOS, whereas Windows is receiving USB device information through a different pathway, and that the firmware has different responses for the different device negotiations.
fn device_model_id_is_rigel(model_id: &String) -> bool {
    return
        model_id.contains("VendorID_10550") &&
        model_id.contains("ProductID_4610")
    ;
}

/// Invokes the `description` message for a passed NSObject and returns the message as a `String`. Invoking the message is unsafe, so be careful only to pass known NSObjects to this function.
fn get_objc_object_desc(ns_object: id) -> String {
    unsafe {
        let desc: id = msg_send![ns_object, description];
        get_string_from_ns_string(desc)
    }
}

/// Converts an NSString to a Rust std::String.
fn get_string_from_ns_string(ns_string: id) -> String {
    unsafe {
        let desc_str = CStr::from_ptr(ns_string.UTF8String()).to_string_lossy().to_string();
        desc_str
    }
}
