
use windows_bindings::{
  Windows::Devices::Enumeration as WinEnumeration,
  Windows::Devices::Usb as WinDevUsb,
  Windows::Foundation
};

use crate::calib::SerializedCalibration;

// RUN THIS TEST VIA: `cargo test test_usb_devices_api -- --nocapture`

#[test]
fn test_usb_devices_api() -> ::windows::Result<()> {
  let vid = crate::usb_info::LEAP_USB_VID;
  let rigel_pid = crate::usb_info::LEAP_USB_PID_RIGEL;
  
  // Get device selector ("Advanced Query Selector") for the Rigel via VID and PID.
  let aqs_filter = WinDevUsb::UsbDevice::GetDeviceSelectorVidPidOnly(vid.into(), rigel_pid.into())?;
  println!("Got aqs filter for device: {}", aqs_filter);
  
  // Naive timeout to wait for async ops to complete "synchronously".
  let timeout = 1_000_000_000u64;
  
  // Find the connected device info via the filter.
  let req = WinEnumeration::DeviceInformation::FindAllAsyncAqsFilter(aqs_filter)?;
  let device_infos = naive_wait_and_get_op_results(&req, timeout)?;
  println!("Found {} devices.", device_infos.Size()?);

  // This keeps returning 0 suddenly for some reason. So..... Let's try another way.

  // Request an enumeration of video devices from WinRT.
  let req = WinEnumeration::DeviceInformation::FindAllAsyncDeviceClass(WinEnumeration::DeviceClass::VideoCapture)?;
  
  // Find the connected Rigel (otherwise panic).
  let device_infos = naive_wait_and_get_op_results(&req, timeout)?;
  let rigel_info = device_infos.into_iter().find(|di| is_device_leap_motion(&di))
      .expect("Failed to find a connected Rigel (is your Rigel connected?)");

  println!("Found connected leap motion device info: {:?}", rigel_info);
  println!("Leap Motion device id is: {:?}", rigel_info.Id()?);

  
  // // // // Note: Naively trying to open the Rigel here is failing. I'm trying to see if there's a "very special way" that Platform is actually opening Rigels.
  // // // //
  // // // // Path: DeviceAttachMonitor.cpp:BuildContextForDevice
  // // // //       -> ConfigureContext line 242
  // // // //       -> DeviceModelManifst::HardwareAbstraction line 61
  // // // //
  // // // // NOTE FOR FUTURE PERIPHERAL SUPPORT: LEAP_USB_PID_PERIPHERAL is handled in the Hardware Abstraction Layer setup as "CyPeripheral" -- Cypress.
  // // // //
  // // // //       case LEAP_USB_PID_RIGEL:
  // // // //         ctxt.Inject<OV580Rigel>(); // Appears to minimally override OV580DeviceBase
  // // // // (OV580 = Omnivision 580.)
  // // // // 
  // // // let device = naive_wait_and_get_op_results(&(WinDevUsb::UsbDevice::FromIdAsync(rigel_info.Id()?)?), timeout)?;
  // // // device.Close()?;

  // // // // Let's try to send a control request.

  // // // let calibration = crate::calib::SerializedCalibration::empty_calibration();
  // // // println!("Initialized an empty calibration: {:?}", calibration);

  // // // let setup_packet = {
  // // //   // Try to define a UVC packet.
  // // //   use crate::WinDevUsb::*;
  // // //   let packet = crate::WinDevUsb::SetupPacket {
  // // //       bm_request_type_recipient: BmRequestTypeRecipient::Interface, // UVC
  // // //       bm_request_type_type: BmRequestTypeType::Class, // Video Class
  // // //       // UVC SetCur -> to Device; otherwise to Host
  // // //       bm_request_type_direction: BmRequestTypeDataPhaseTransferDirection::DeviceToHost, 
  // // //       // UVC request code, here GetCur, 0x81 == 192.
  // // //       b_request: 0x81,
  // // //       // w_value: CS or Command Selector.
  // // //       // The command Selector. Leap XU selectors are u8s, and get shifted for LSB endianness (I think? The shift << 8 DEFINITELY happens.)
  // // //       // Here we pick 25: LEAP_XU_CALIBRATION_DATA in leap_xu.h.
  // // //       w_value: 25,
  // // //       // w_index: UVC Endpoint Terminal Index
  // // //       // When Platform constructs a WinUVCXURequest (WinUVCInterface.cpp:54), the constructor takes an XUEntry, whose "selector" goes from:
  // // //       // (u8 << 8) => "int" (int32? int64?) => truncated to u16
  // // //       // This is then used as the endpoint terminal index.
  // // //       // This value is supposed to be "Extension Unit ID and Interface".
  // // //       // So WHAT IS the XUEntry for calibration?
  // // //       // This data for every XUEntry is from WinUVCInterface.cpp:88:
  // // //       // m_usbDevice->GetCurrentConfigurationDescriptor().data()
  // // //       //              ^ std::vector<uint8>                 ^ vec to pointer
  // // //       // '--> reinterpret_cast the bytes to a usb_config_descriptor, defined in USBDescriptor.h:186. A USB Config Descriptor is a USB spec thing.
  // // //       // A UVCDescriptor from UVCDescriptor.h:22 is build by PARSING the bytes of a
  // // //       // USB Configuration Descriptor.
  // // //       // This parsing begins in UVCDescriptor.cpp:128 Init().
  // // //       // For a Rigel, this parsing seems like it will find TWO Video Control Extension Unit Descriptors, based on looking at USBView for my Rigel, which shows two.
  // // //       // These descriptors are identified in Platform for the w_index field by looking at the bUnitID entry in the descriptor. This is the Extension Unit ID, and the format for this appears to be defined as a "UVC XU Descriptor Header".
  // // //       // The XU Unit ID we want -- possibly for only the specific Rigel I have plugged in? -- is mapped from GUID to the Unit ID.
  // // //       // The Unit ID we want is: 0xE0. (USBView Rigel; UVCDescriptor.cpp:25)
  // // //       // The GUID is given ALSO in the VC XU Unit Descriptor. It's used... nowhere.
  // // //       // It appears to be completely unused.
  // // //       // Here we manually provide 0xE0, the bUnitID for the Extension Unit we want.
  // // //       w_index: 0xE0,
  // // //       // Number of bytes of data to send, here sizeof(SerCalib)
  // // //       w_length: std::mem::size_of::<SerializedCalibration>() as u16, 
  // // //   };

  // // //   let packet_bytes = packet.pack().expect("Couldn't pack setup packet.");
  // // //   println!("Constructed setup packet to retrieve calibration via XU: {:?}", packet_bytes);

  // // //   // packet_bytes

  // // // };

  // Send a control transfer!

  

  // let init_setup_packet = {
  //   let packet = WinDevUsb::UsbSetupPacket::new()?;
  //   // packet.SetRequest()
  // };
  
  // println!("OK, got device info... {:?}", device);
  
  // device.Close()?;
  Ok(())
}

/// Vendor ID for Leap Motion. Leap Motion (now Ultraleap) camera devices contain this in their USB device ID string.
pub const VENDOR_ID__LEAP_MOTION: &'static str = "VID_2936";
/// Older Vendor ID for LEAP Motion. The Peripheral (Leap Motion Controller) may contain this in its USB device ID string.
pub const VENDOR_ID__LEAP_MOTION_OLDER: &'static str = "VID_F182";
/// Product ID for the Rigel, AKA the SIR 170. Rigel / SIR 170 devices contain this in their USB device ID string.
pub const PRODUCT_ID__RIGEL     : &'static str = "PID_1202";

/// Checks the DeviceInformation ID string to see if it contains the Leap Motion vendor ID and Rigel (aka SIR 170) product ID. If it does, returns true, otherwise returns false.
fn is_device_rigel(device_info: &WinEnumeration::DeviceInformation) -> bool {
  let device_id_str = device_info.Id().map(|hstr| hstr.to_string()).unwrap_or_default();
  
  device_id_str.contains(VENDOR_ID__LEAP_MOTION) &&
  device_id_str.contains(PRODUCT_ID__RIGEL)
}

fn is_device_leap_motion(device_info: &WinEnumeration::DeviceInformation) -> bool {
  let device_id_str = device_info.Id().map(|hstr| hstr.to_string()).unwrap_or_default();
  
  device_id_str.contains(VENDOR_ID__LEAP_MOTION) ||
  device_id_str.contains(VENDOR_ID__LEAP_MOTION_OLDER)
}

/// Spin in place and get the async op results.
fn naive_wait_and_get_op_results<T: ::windows::RuntimeType>(async_op: &Foundation::IAsyncOperation<T>, loop_limit: u64) -> ::windows::Result<T> {
  naive_wait_for_async_op(async_op, loop_limit).unwrap();
  async_op.GetResults()
}

/// Spin in place waiting for the async operation to complete. Not very graceful.
///
/// TODO: Use a time-based timeout and not a CPU-clock-based timeout.
// fn naive_wait_for_device<T>(device_req: &IAsyncOperation<T>, loop_limit: u64) -> Result<(), &'static str> {
fn naive_wait_for_async_op<T: windows::RuntimeType>(async_op: &Foundation::IAsyncOperation<T>, loop_limit: u64) -> std::result::Result<(), &'static str> {
  let mut loops = 0u64;
  loop {
    if async_op.Status().unwrap() == Foundation::AsyncStatus::Completed {
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
fn naive_wait_for_async_act(req: &Foundation::IAsyncAction, loop_limit: u64) -> std::result::Result<(), &'static str> {
  let mut loops = 0u64;
  loop {
    if req.Status().unwrap() == Foundation::AsyncStatus::Completed {
      break;
    }
    
    loops += 1; if loops >= loop_limit { break; }
  }
  if loops >= loop_limit { return Err("Timeout while waiting for the IAsyncAction to complete.")}
  
  Ok(())
}
  
  // Notes
  
  // Initialization: We need an identifier for a Leap USB device.
  
  // Enumeration: fn() -> [LeapUSBDeviceDescription]
  // LeapUSBDeviceDescription: (USBDeviceID, **)
  
  // Initialization: fn(USBDeviceID) -> USBDevice
  // (USBDevice has UVCInterface)
  // UVC Control: fn(UVCInterface)
  // Shutdown: fn(USBDevice)
  
  // Use Leap-specific Rigel-specific construction, first pass.
  
  // libusb's USB device/device handle abstraction doesn't work for Leap Motion devices (or at least, Rigels) because the device doesn't exactly fit the spec -- it reports frame sizes as a video device that doesn't actually work. I believe libusb's Windows backend tries to open the device with some default configuration that the device declares, but this doesn't work -- you have to choose a specific configuration BEFORE trying to open the device.
  // The Windows API allows this, but libusb does not -- arguably this is reasonable, but it breaks support for the Rigel as-is. Boy, we should really fix this thing's firmware. Oh well!
  
  // ---
