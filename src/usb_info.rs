
/// The Leap Motion or Ultraleap USB vendor identifier.
/// 
/// 0x2936. Base 10: 10550.
pub const LEAP_USB_VID: u16 = 0x2936; 
/// Leap Motion or Ultraleap PID as hex value in a string.
pub const LEAP_USB_VID_STR: &str = "2936";
/// `VID_2936`
pub const LEAP_USB_VID_STR_WITH_PREFIX: &str = "VID_2936";

/// An old but still sometimes observed LEAP Motion USB vendor identifier.
/// 
/// 0x2936. Base 10: 10550.
pub const LEAP_USB_VID_OLD: u16 = 0xF182; 
/// Old Leap Motion or Ultraleap PID as hex value in a string.
pub const LEAP_USB_VID_OLD_STR: &str = "F182";
/// `VID_F182`
pub const LEAP_USB_VID_OLD_STR_WITH_PREFIX: &str = "VID_F182";

/// The UVC product identifier for the Leap Motion Controller, aka LMC, aka Peripheral.
/// 
/// 0x0003.
pub const LEAP_USB_PID_PERIPHERAL: u16 = 0x0003;
/// Peripheral PID as hex value in a string.
pub const LEAP_USB_PID_PERIPHERAL_STR: &str = "0003";

/// The UVC product identifier for the Ultraleap SIR170, aka Rigel.
/// 
/// 0x1202. Base 10: 4610.
pub const LEAP_USB_PID_RIGEL: u16 = 0x1202;

/// Returns a descriptive name for the argument Ultraleap USB/UVC Vendor or Product ID.
pub const fn friendly_name_from_usb_id(pid_or_vid: u16) -> &'static str {
  match pid_or_vid {
    LEAP_USB_VID => return "Leap Motion (Ultraleap)",
    LEAP_USB_PID_PERIPHERAL => return "Leap Motion Controller aka Peripheral",
    LEAP_USB_PID_RIGEL => return "SIR 170 aka Rigel",
    _ => return "Unknown Vendor or Product ID"
  }
}
