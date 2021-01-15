// tinyrigel

// use bindings_winrt::*;
// use winrt;

/// Vendor ID for Leap Motion. Leap Motion (now Ultraleap) camera devices contain this in their USB device ID string.
const VENDOR_ID__LEAP_MOTION: &'static str = "VID_2936";
/// Product ID for the Rigel, AKA the SIR 170. Rigel / SIR 170 devices contain this in their USB device ID string.
const PRODUCT_ID__RIGEL     : &'static str = "PID_1202";

#[cfg(test)]
#[cfg(windows)]
mod tests_windows_winrt {
    use bindings_winrt::*;
    use windows::{devices::enumeration::{DeviceClass, DeviceInformation, DeviceInformationCollection}, media::capture};
    use winrt::{self, foundation::IAsyncOperation};

    use crate::{PRODUCT_ID__RIGEL, VENDOR_ID__LEAP_MOTION};

    #[test]
    fn can_enumerate_video_devices() -> Result<(), &'static str> {
        println!("\n## can_enumerate_video_devices ##");

        // Request an enumeration of video devices from WinRT.
        let device_req = DeviceInformation::find_all_async_device_class(DeviceClass::VideoCapture);
        if device_req.is_err() { return Err("Failed to initiate device enumeration.") }
        let device_req = device_req.unwrap();

        // Spin in-place while we wait for the request to finish...
        naive_wait_for_device(&device_req, 1_000_000u64).expect("Wait for device");

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

    /// Spin in place waiting for the async request to complete. Not very graceful.
    ///
    /// ODO: Use a time-based timeout and not a CPU-clock-based timeout.
    fn naive_wait_for_device(device_req: &IAsyncOperation<DeviceInformationCollection>, loop_limit: u64) -> Result<(), &'static str> {
        let mut i = 0u64;
        loop {
            if device_req.status().unwrap() == winrt::foundation::AsyncStatus::Completed {
                break;
            }
    
            i += 1; if i >= loop_limit { break; }
        }
        if i >= loop_limit { return Err("Timeout while waiting for the device request to complete."); }

        Ok(())
    }

    #[test]
    fn can_retrieve_rigel_frame() -> Result<(), &'static str> {
        println!("\n## can_retrieve_rigel_frame ##");

        // Get connected devices.
        let device_req = DeviceInformation::find_all_async_device_class(DeviceClass::VideoCapture)
            .expect("Failed to request VideoCapture devices");
        naive_wait_for_device(&device_req, 1_000_000u64)
            .expect("Failed while waiting for device enumeration");

        // Find the connected Rigel (otherwise panic).
        let device_infos: DeviceInformationCollection = device_req.get_results().unwrap();
        let rigel_device_idx = device_infos.into_iter().position(|di| is_device_rigel(&di))
            .expect("Failed to find a connected Rigel (is your Rigel connected?)");

        // Use the found index to initialize a MediaCapture object for the Rigel.
        let mut media_cap_init_settings = capture::MediaCaptureInitializationSettings::new()
            .expect("Failed to construct MediaCaptureInitializationSettings.");
        {
            media_cap_init_settings.set_video_device_id(rigel_device_idx);
        }

        println!("TODO: Use ID to set up MediaCapture init settings");

        Ok(())
    }

    fn is_device_rigel(device_info: &DeviceInformation) -> bool {
        let device_id_str = device_info.id().map(|hstr| hstr.to_string()).unwrap_or_default();
        
        device_id_str.contains(VENDOR_ID__LEAP_MOTION) &&
        device_id_str.contains(PRODUCT_ID__RIGEL)
    }

}
