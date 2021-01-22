// tests/tests_macos.rs
//
// This file may look funny, as using the objc crate to cross-communicate with Objective-C involves heavy use of the msg_send![] macro. This macro allows interoperable calls to Objective-C methods on Objective-C objects, and mimics the syntax of such Objective-C method calls.
//
// Such calls don't have many safety guarantees and type annotations need to be provided as hints to the macro to correctly link to the appropriate methods. As such, this code is more brittle and unsafe than standard (safe) Rust.
//
// Check out the objc crate for more info.

use std::{ffi::{CStr}, ptr};
use cocoa_foundation::{base::id, base::{BOOL, NO, YES, nil}, foundation::{NSInteger, NSString}};
use core_foundation::{base::TCFType, error::{CFError, CFErrorRef}};
use objc::{class, declare::ClassDecl, msg_send, runtime::{Object, Sel}, sel, sel_impl};
use ptr::null;
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

// TODO: REMOVEME? If 'id' works for the CMSampleBufferRef instead
// // Linked opaque struct types.
// extern "C" {
//     type CMSampleBufferRef;
// }
// unsafe impl Encode for CMSampleBufferRef {
//     fn encode() -> objc::Encoding {
//         todo!()
//     }
// }
// // opaqueCMSampleBuffer*
// // CM_BRIDGED_TYPE(id) implies to me that it's OK to replace the type definition with id, so I'm going to try that.

// Dispatch
//
// // Needed for dispatch_queue_create.
// #[link(name = "Dispatch", kind = "framework")]
// extern "C" {
// }

// CoreMedia-related structs.
#[repr(C)]
pub struct CMVideoDimensions {
    pub width: i32,
    pub height: i32,
}

// Tests
// ---

// TODO: Add "backends/macos-xcode-ref"

#[test]
fn can_retrieve_rigel_frame() -> Result<(), &'static str> {
    println!("=== can_retrieve_rigel_frame ===");

    // Attempt to enumerate devices and retrieve a connected Rigel.
    let rigel_device: Option<id> = unsafe {
        // NSArray<AVCaptureDevice *> *
        let devices: id = msg_send![class!(AVCaptureDevice), devices];
        let devices_count: NSInteger = msg_send![devices, count];
        println!("AVCaptureDevice.devices devices_count is {}", devices_count);

        let mut rigel: id = nil;
        for device_idx in 0..devices_count {
            let device: id = msg_send![devices, objectAtIndex: device_idx];
            let model_id: id = msg_send![device, modelID];
            let model_id_str = CStr::from_ptr(model_id.UTF8String()).to_string_lossy().to_string();
            println!("Device model ID... {}", &model_id_str);

            if device_model_id_is_rigel(&model_id_str) {
                println!("Found Rigel. Model id was: {}", model_id_str);
                rigel = device;
                break;
            }
        }

        if rigel != nil {
            Some(rigel)
        } else {
            None
        }
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
    let capture_session: id = unsafe {
        let allocated_capture_session: id = msg_send![class!(AVCaptureSession), alloc];
        if allocated_capture_session == nil { return Err("Failed to allocate a new AVCaptureSession."); }

        msg_send![allocated_capture_session, init]
    };
    println!("Trying AVCaptureSession canAddInput");
    let can_add_rigel_input: BOOL = unsafe { msg_send![capture_session, canAddInput: rigel_input] };
    if can_add_rigel_input == NO {
        return Err("Unable to add the rigel_input node to the new capture_session.");
    }
    println!("Trying AVCaptureSession addInput");
    unsafe { let _: () = msg_send![capture_session, addInput: rigel_input]; }
    println!("Added the rigel_input node to the new capture_session.");

    // Initialize the frame callback output node with "YUY2" format. (The data we'll get is only pretending to be YUY2, but that's OK!)
    //
    // First, declare the delegate class.
    //
    // Currently using as a reference:
    // https://github.com/ndarilek/tts-rs/blob/d3e05b5a7a642eb3212528ecc8cdedd406673213/src/backends/av_foundation.rs
    let capture_output: id = unsafe {
        let allocated_capture_output: id = msg_send![class!(AVCaptureVideoDataOutput), alloc];
        if allocated_capture_output == nil { return Err("Failed to allocate a new AVCaptureVideoDataOutput."); }

        msg_send![allocated_capture_output, init]
    };
    // We have to provide a dispatch queue to the output capture node.
    let dispatch_queue = unsafe { dispatch_queue_create(null(), nil /* DISPATCH_QUEUE_SERIAL -- to guarantee order. Xcode reveals this is actually defined to be NULL, so let's hope that never changes! */) };
    // We also specify that we just want to discard late frames.
    unsafe { let _: () = msg_send![capture_output, setAlwaysDiscardsLateVideoFrames: YES]; }

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
        // CVImageBufferRef imageBuffer = CMSampleBufferGetImageBuffer(sampleBuffer);
        // if (!imageBuffer) {
        //     return;
        // }
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
        
        // size_t bytesPerRow = CVPixelBufferGetBytesPerRow(imageBuffer);
        // //size_t width = CVPixelBufferGetWidth(imageBuffer);
        // size_t height = CVPixelBufferGetHeight(imageBuffer);
        // void *src_buff = CVPixelBufferGetBaseAddress(imageBuffer);
        // NSData *data = [NSData dataWithBytes:src_buff length:bytesPerRow * height];
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

            println!("src_buf_as_slice is {} bytes in length.", src_buf_as_slice.len());

            const FRAME_YUY2_BYTES_PER_PIXEL: usize = 2;
            const FRAME_PX_COUNT: usize = 384 * 384; // 147,456
            const FRAME_NUM_BYTES: usize = FRAME_PX_COUNT * FRAME_YUY2_BYTES_PER_PIXEL;

            println!("We expect there to be {} bytes per frame.", FRAME_NUM_BYTES);

            if src_buf_as_slice.len() != FRAME_NUM_BYTES {
                println!("Unexpected length mismatch, src_buf {} bytes != FRAME_NUM_BYTES of {} bytes.", src_buf_as_slice.len(), FRAME_NUM_BYTES);
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
                println!("Non-zero return from CVPixelBufferLockBaseAddress: {}", cv_return);
                return;
            }
        }
        
        // From here, we have everything we need accessible from safe code.

        // Here we would want to sync with the main thread to get an OK-to-write...
        // TODO...

        // Save the image as a PNG as a test.
        use image::GenericImageView;
        let mut img = image::DynamicImage::new_luma8(384 * 2, 384);
        let img_luma8 = img.as_mut_luma8().unwrap();
        img_luma8.copy_from_slice(copied_frame_data.as_slice());
        println!("Copied image from frame data, {}x{}", img.width(), img.height());
        img.save("test.bmp").unwrap();
        println!("Invoked write to test.bmp");

        // Here we would want to transmit a "done" signal...
        // TODO...

        // const size_t row = bytesPerRow;
        // const size_t halfRow = bytesPerRow * 0.5;
        // UInt8 b0, b1, b2, b3, b4, b5, b6, b7;
        // [data getBytes:&b0 range:NSMakeRange(00 * row + halfRow, sizeof(UInt8))];
        // [data getBytes:&b1 range:NSMakeRange(10 * row + halfRow, sizeof(UInt8))];
        // [data getBytes:&b2 range:NSMakeRange(20 * row + halfRow, sizeof(UInt8))];
        // [data getBytes:&b3 range:NSMakeRange(30 * row + halfRow, sizeof(UInt8))];
        // [data getBytes:&b4 range:NSMakeRange(40 * row + halfRow, sizeof(UInt8))];
        // [data getBytes:&b5 range:NSMakeRange(50 * row + halfRow, sizeof(UInt8))];
        // [data getBytes:&b6 range:NSMakeRange(60 * row + halfRow, sizeof(UInt8))];
        // [data getBytes:&b7 range:NSMakeRange(70 * row + halfRow, sizeof(UInt8))];
        // NSLog(@"[Frame Thread] Some bytes: %d %d %d %d %d %d %d %d",
        //     b0, b1, b2, b3, b4, b5, b6, b7
        // );
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
    let frame_delegate_obj: id = unsafe { msg_send![frame_delegate_cls, new] };
    unsafe {
        let _: () = msg_send![capture_output, setSampleBufferDelegate: frame_delegate_obj queue: dispatch_queue];
    }

    // We always want to set the capture_output videoSettings pixel format to 846624121, aka '2vuy', retrievable from the declaration kCVPixelFormatType_422YpCbCr8.
    // ........ we're going to try skipping this step because it would be a major pain.
    //
    // [captureOutput setVideoSettings:[NSDictionary dictionaryWithObject: [NSNumber numberWithInt:kCVPixelFormatType_422YpCbCr8] forKey:(id)kCVPixelBufferPixelFormatTypeKey]];
    // NSLog(@"Set captureOutput videoSettings pixel format to %u, aka '2vuy', via kCVPixelFormatType_422YpCbCr8.", kCVPixelFormatType_422YpCbCr8);

    // Add the output node, with the frame callback delegate, to the capture session.
    let can_add_capture_output: BOOL = unsafe { msg_send![capture_session, canAddOutput: capture_output] };
    if can_add_capture_output == NO {
        return Err("Unable to add capture_output to capture_session.");
    }
    unsafe { let _: () = msg_send![capture_session, addOutput: capture_output]; }
    println!("Added output with frame callback delegate to capture_session.");

    // Start the capture session and unlock the device configuration.
    unsafe {
        let _: () = msg_send![capture_session, startRunning];
        let _: () = msg_send![rigel_device, unlockForConfiguration];
    }
    println!("Unlocked the Rigel configuration.");

    // Confirm whether the capture_session is running.
    let is_running: BOOL = unsafe { msg_send![capture_session, isRunning] };
    if is_running == NO {
        return Err("Capture session failed to be running after invoking startRunning.");
    }
    println!("Capture session is running...");

    // Spin for a bit...
    let mut i = 0;
    for _ in 0..2_000_000_000u64 {
        i += 1;
    }
    println!("{}", i);
    for _ in 0..100000 {
      let is_running: BOOL = unsafe { msg_send![capture_session, isRunning] };
      println!("Capture session is running? {}", is_running);
    }

    // Stop...
    unsafe { let _: () = msg_send![capture_session, stopRunning]; }
    println!("Invoked stopRunning on capture_session.");

    // TODO: Synchronization plan might not be easy, because the callback thread isn't a Rust closure, but a function with a pointer. Probably have to use a lazy_static! mutex, which is kind of non-ideal.
    //
    // Maybe there's a lock-free lazy_static! queue of frame buffers I can write to?
    //
    // Or maybe there's a better way...
    //
    // Original plan:
    // ---
    // Wait to sync with the callback thread and give it the OK to copy the frame data, convert to an image format, and write to disk.
    //
    // Wait to sync with the the callback thread, to have it report that it finished writing to disk.

    // HUGE TODO HERE:
    // Referencing: https://github.com/SSheldon/rust-objc/blob/6092caa90ca0622b82ea1ebb820a614db2cee82b/examples/example.rs
    // for use of "StrongPtr" and "WeakPtr" to get "ARC-like semantics".
    //
    // Additionally, see: https://github.com/SSheldon/rust-objc#reference-counting
    //
    // So when I am doing new, or alloc/init, for constructing objects, I need to be returning StrongPtrs to them so that when they are dropped, Release can be called.

    // Clean up.
    unsafe { let _: () = msg_send![capture_session, removeInput: rigel_input]; }
    unsafe { let _: () = msg_send![capture_session, removeOutput: capture_output]; }

    // No... this doesn't seem like it's stable...
    // Let's just use StrongPtrs as above :(
    // // For now.. do we release every object manually?
    // // Objects:
    // //  - rigel_device
    // //  - format_384x384_90fps
    // //  - rigel_input
    // //  - capture_session
    // //  - capture_output
    // //  - frame_delegate_obj
    // unsafe {
    //     let _: () = msg_send![frame_delegate_obj, release];
    //     let _: () = msg_send![capture_output, release];
    //     let _: () = msg_send![capture_session, release];
    //     let _: () = msg_send![rigel_input, release];
    //     let _: () = msg_send![format_384x384_90fps, release];
    //     let _: () = msg_send![rigel_device, release];
    // }

    // Without ARC, release the dispatch queue too.
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
