
use std::os::windows::raw::HANDLE as Win32HANDLE;

use windows_bindings::{
  Windows::Win32::Devices::Usb as Win32DevUsb,
  Windows::Win32::Devices::DeviceAndDriverInstallation as Win32Device,
  Windows::Win32::Foundation as Win32Foundation,
  Windows::Win32::System::Diagnostics::Debug as Win32Debug,
  Windows::Win32::Storage::FileSystem as Win32FileSystem,
};

// This returns e.g. my logitech USB camera.
// ImagingDeviceClassGUID: `6BDD1FC6-810F-11D0-BEC7-08002BE2092F`
const imaging_device_class_guid: windows::Guid = windows::Guid::from_values(
  0x6BDD1FC6, 0x810F, 0x11D0, [0xBE, 0xC7, 0x08, 0x00, 0x2B, 0xE2, 0x09, 0x2F], 
);
// This returns (surprise) all the connected USB cameras! But not imaging devices, which is a different thing apparently.
// CameraDeviceClassGUID: `CA3E7AB9-B4C3-4AE6-8251-579EF933890F`
const camera_device_class_guid: windows::Guid = windows::Guid::from_values(
  0xCA3E7AB9, 0xB4C3, 0x4AE6, [0x82, 0x51, 0x57, 0x9E, 0xF9, 0x33, 0x89, 0x0F]
);
// USBSerialDeviceClassGUID: `4d36e978-e325-11ce-bfc1-08002be10318`
// On my machine these appear to not be related to any Leap devices.
const usb_serial_device_class_guid: windows::Guid = windows::Guid::from_values(
  0x4D36E978, 0xE325, 0x11CE, [0xBF, 0xC1, 0x08, 0x00, 0x2B, 0xE1, 0x03, 0x18]
);
// LeapUSBDeviceInterfaceGUID: `2B1A1B59-D3AC-4841-AA28-4819A66D7CB9`
// This is an interface GUID, not a device class GUID.
const leap_usb_device_interface_guid: windows::Guid = windows::Guid::from_values(
  0x2B1A1B59, 0xD3AC, 0x4841, [0xAA, 0x28, 0x48, 0x19, 0xA6, 0x6D, 0x7C, 0xB9]
);
// TeensyUSBControllerGUID `40908837-F958-4025-B3BF-0A0FD990DDF0`

// RUN THIS TEST VIA: `cargo test winusb_stuff -- --nocapture`
#[test]
pub fn winusb_stuff() -> windows::Result<()> {

  // Let's try to use the Win32 Setup API (yes... really...) to get a list of device handles...
  let leap_dev_infoset: Win32HANDLE = unsafe {
     Win32Device::SetupDiGetClassDevsA(
      &camera_device_class_guid,
      Win32Foundation::PSTR::NULL, /*  */
      Win32Foundation::HWND::NULL, /* Win32Foundation::HWND::NULL */
      Win32Device::DIGCF_PRESENT
    )
  };
  if leap_dev_infoset.is_null() {
    println!("Device info is null.")
  } else {
    println!("Got device info handle.")
  }

  // Enumerate info data. Find a Leap device.
  let leap_device_pnpid: String;
  let mut leap_device_pnpid_u16s: Option<Vec<u16>> = None;
  println!("Setting SP_DEVINFO_DATA size as {}, expect 32UL.", std::mem::size_of::<Win32Device::SP_DEVINFO_DATA>() as u32);
  let mut leap_dev_info_data = Win32Device::SP_DEVINFO_DATA {
    cbSize: std::mem::size_of::<Win32Device::SP_DEVINFO_DATA>() as u32,
    ..Default::default()
  };
  let mut dev_member_idx = 0u32;
  let mut leap_dev_member_idx = !0u32;
  // let device_bus_id =
  loop {
    // Get info data.
    let has_info_data = unsafe { Win32Device::SetupDiEnumDeviceInfo(
      leap_dev_infoset,
      dev_member_idx,
      &mut leap_dev_info_data
    )};
    if !has_info_data.as_bool() {
      // We're good if we just hit the end.
      if unsafe { Win32Debug::GetLastError() == Win32Debug::ERROR_NO_MORE_ITEMS } {
        println!("OK, Hit the end of items.");
        break;
      }
      // Otherwise print the error.
      println!("Error getting info data: {}", windows::HRESULT::from_thread().message());
      break;
    }

    // platform/DeviceEnumeratorWinUSB.cpp:122
    // Get device instance identifier -- this is the hardware bus identifier.
    println!("Calling SetupDiGetDeviceInstanceIdW...");
    const dev_inst_id_max_path: u32 = Win32Foundation::MAX_PATH; // 260u32
    println!("- max path is {}", dev_inst_id_max_path);
    let dev_inst_id_arr = [0u16; dev_inst_id_max_path as usize];
    println!("- size of arr is {:?}", std::mem::size_of_val(&dev_inst_id_arr));
    let mut dev_inst_id_arr_bytes: [u64; 2];
    dev_inst_id_arr_bytes = unsafe { std::mem::transmute(
      &dev_inst_id_arr as *const [u16]
    )};
    println!("- the transmuted bytes are {:?}", dev_inst_id_arr_bytes);
    println!("- Expect size to be 16 * 260u32 = 4160");
    let mut required_size: u32 = 0; // Must pass a pointer to this =_=
    let dev_inst_id = unsafe {
      Win32Foundation::PWSTR(dev_inst_id_arr_bytes[0] as *mut u16)
    };
    let got_dev_inst_id = unsafe { Win32Device::SetupDiGetDeviceInstanceIdW(
      leap_dev_infoset,
      &mut leap_dev_info_data,
      dev_inst_id,
      dev_inst_id_max_path,
      /* DWORD aka u32 */ &mut required_size
    )};
    if !got_dev_inst_id.as_bool() {
      println!("Error getting dev inst id: {} (continuing...)", windows::HRESULT::from_thread().message());
      continue
    } else {
      // let str: &String = dev_inst_id.into();
      println!("OK got PWSTR dev instance ID: {:?}", dev_inst_id);
      println!("The required number of elements was: {}", required_size);

      let u16_str = unsafe { widestring::U16String::from_ptr(dev_inst_id.0, required_size as usize) };
      // This is the hardware bus identifier for the device!
      // This is IDENTICAL to the string we get if we instead use the Windows::Devices::Enumeration API while enumerating over the VideoCapture device class.
      let dev_id_str = u16_str.to_string_lossy();
      let u16s = u16_str.as_slice();
      println!("PWSTR contents: {}", dev_id_str);

      if dev_id_str.contains(crate::usb_info::LEAP_USB_VID_STR) || dev_id_str.contains(crate::usb_info::LEAP_USB_VID_OLD_STR) {
        leap_device_pnpid = dev_id_str;
        leap_device_pnpid_u16s = Some(Vec::from(u16s));

        leap_dev_member_idx = dev_member_idx;
        println!("OK, leap_dev_member_idx is {}", leap_dev_member_idx);

        break;
      }
    }

    // platform/DeviceEnumeratorWinUSB.cpp converts the device information into a "SinglePNPID" which contains:
    // - The Device Class (a custom enum in platform) -- akin to Windows DeviceClass
    // - !!!!! The actual hardware bus identifier string called "pnpid" !!!!!
    //   - pnpid is EXTREMELY important and is used EVERYWHERE to identify the device!
    //   - This is IDENTICAL to the Windows::Devices::Enumeration API key, see:
    //     - 
    // - friendlyName -- print-friendly device name. Windows provides this
    // - VID -- Vendor ID obv
    // - PID -- Product ID obv, all provided by easier Windows APIs
    // - REV -- Apparently this was a thing at one point. Who cares. Don't need it
    
    dev_member_idx += 1;
  }
  if dev_member_idx == 0 { println!("Oh no! Didn't get any device info data."); }

  // Try getting interfaces since we got a leap device
  println!("Have leap device now in info_data");

  unsafe {
    Win32Debug::SetLastError(0); // Clear last error
    print_last_error();

    let mut leap_dev_interface_idx = 0u32;

    println!("Trying to get interface data via SetupDiEnumDeviceInterfaces.");

    loop {
      let mut out_dev_interface_data = Win32Device::SP_DEVICE_INTERFACE_DATA {
        cbSize: std::mem::size_of::<Win32Device::SP_DEVICE_INTERFACE_DATA>() as u32,
        ..Default::default()
      };
      let res = Win32Device::SetupDiEnumDeviceInterfaces(
        leap_dev_infoset,
        &mut leap_dev_info_data,

        // &Win32DevUsb::GUID_DEVINTERFACE_USB_DEVICE,
        &leap_usb_device_interface_guid,

        leap_dev_interface_idx,
        &mut out_dev_interface_data
      );
      if !res.as_bool() && Win32Debug::GetLastError() != Win32Debug::ERROR_NO_MORE_ITEMS {
        println!("Failed to get device interface data.");
        print_last_error();
        break;
      }
      else if !res.as_bool() && Win32Debug::GetLastError() == Win32Debug::ERROR_NO_MORE_ITEMS {
        println!("Done enumerating interfaces.");
        break;
      }
      else {
        println!("OK, got some interface data: {:?}", out_dev_interface_data);
      }

      leap_dev_interface_idx += 1;
    }
  }

  // OK. To get a device handle (CLOSE IT AFTER OPENING):
  // - You use the Create File API. yeah.
  // Win32FileSystem

  // Nope! Peripherals use Property Knocking and Rigels use DirectShow for UVC transport. So this WinUSB-based method is going to fail.
  // STUFF THAT FAILS //
  // // // if leap_device_pnpid_u16s.is_none() {
  // // //   println!("Couldn't find a Leap Motion device, so can't continue.");
  // // // }
  // // // let leap_device_pnpid_u16s = leap_device_pnpid_u16s.unwrap();

  // // // let dev_handle = unsafe { Win32FileSystem::CreateFileW(
  // // //   pwstr_from_u16_arr(leap_device_pnpid_u16s.as_slice()),
  // // //   Win32FileSystem::FILE_GENERIC_READ | Win32FileSystem::FILE_GENERIC_WRITE,
  // // //   Win32FileSystem::FILE_SHARE_READ | Win32FileSystem::FILE_SHARE_WRITE,
  // // //   0 as _ /* nullptr */,
  // // //   Win32FileSystem::OPEN_EXISTING,
  // // //   Win32FileSystem::FILE_ATTRIBUTE_NORMAL | Win32FileSystem::FILE_FLAG_OVERLAPPED,
  // // //   Win32Foundation::HANDLE::NULL,
  // // // )};
  // // // if dev_handle.is_invalid() {
  // // //   println!("Tried to create a file handle for I/O with the Leap Motion device, but it failed.");
  // // //   print_last_error();
  // // // }

  // OK, instead, let's try to initialize DirectShow stuff.

  // TODO: Pick up from UVCTransportDShow.cpp:719 or so. Follow the DirectShow logic for setting up whatever we need to get Rigel stuff!

  // Destroy the device INFO handle.
  if !leap_dev_infoset.is_null() { unsafe {
    let res = Win32Device::SetupDiDestroyDeviceInfoList(leap_dev_infoset);
    if res.as_bool() { println!("OK, destroyed device handle."); }
    else { println!("Oh no! Handle not destroyed!"); }
  }}

  Ok(())
}

fn pwstr_from_u16_arr(u16_arr: &[u16]) -> Win32Foundation::PWSTR {
  println!("- size of arr is {:?}", std::mem::size_of_val(&u16_arr));
  let mut u16_arr_bytes: [u64; 2];
  u16_arr_bytes = unsafe { std::mem::transmute(
    u16_arr as *const [u16]
  )};
  println!("- the transmuted bytes are {:?}", u16_arr_bytes);
  let mut required_size: u32 = 0; // Must pass a pointer to this =_=
  let pwstr = unsafe {
    Win32Foundation::PWSTR(u16_arr_bytes[0] as *mut u16)
  };

  // For debugging, print the pwstr before returning it.
  let u16_str = unsafe { widestring::U16String::from_ptr(pwstr.0, u16_arr.len()) };
  // This is the hardware bus identifier for the device!
  // This is IDENTICAL to the string we get if we instead use the Windows::Devices::Enumeration API while enumerating over the VideoCapture device class.
  let str = u16_str.to_string_lossy();
  println!("Converted u16 arr to pwstr; round-trip contents are: {}", str);

  pwstr
}

fn print_last_error() {
  println!("Last error: {}", windows::HRESULT::from_thread().message());
}
