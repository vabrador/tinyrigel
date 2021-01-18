// tests/tests_macos.rs

use std::ffi::{CStr};

use cocoa_foundation::{base::id, base::nil, foundation::{NSInteger, NSString}};
// use objc::runtime::Object; <-- id is short-hand for objc::runtime::Object.
use objc::{class, msg_send, sel, sel_impl};

// These tests are only valid on macOS and iOS and require AVFoundation to be linked in.
#[cfg(any(target_os = "macos", target_os = "ios"))]
#[link(name = "AVFoundation", kind = "framework")]

// TODO: Add "backends/macos-xcode-ref"

#[test]
fn can_enumerate_video_devices() -> Result<(), &'static str> {
    println!("=== can_enumerate_video_devices ===");

    // Attempt to enumerate devices and retrieve a connected Rigel.
    let rigel_device: Option<id> = unsafe {
        let devices: id = msg_send![class!(AVCaptureDevice), devices];
        let devices_count: NSInteger = msg_send![devices, count];
        println!("AVCaptureDevice.devices devices_count is {}", devices_count);

        let mut rigel: id = nil;
        for device_idx in 0..devices_count {
            let device: id = msg_send![devices, objectAtIndex:device_idx];
            let model_id: id = msg_send![device, modelID];
            let model_id_str = CStr::from_ptr(model_id.UTF8String()).to_string_lossy().to_string();
            println!("Device model ID... {}", &model_id_str);

            if device_model_id_is_rigel(&model_id_str) {
                println!("Found Rigel. Model id was: {}", model_id_str);
                rigel = device;
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

    // To check whether this reference is still valid after the unsafe block, let's hop back in and try to print its description.
    unsafe {
        let desc: id = msg_send![rigel_device, description];
        let desc_str = CStr::from_ptr(desc.UTF8String()).to_string_lossy().to_string();
        println!("Rigel device NSObject description: {}", desc_str);
    }

    Ok(())
}

fn device_model_id_is_rigel(model_id: &String) -> bool {
    return
        model_id.contains("VendorID_10550") &&
        model_id.contains("ProductID_4610")
    ;
}
