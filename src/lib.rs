// tinyrigel

// TODO: This was needed for macOS, may not be relevant anymore.
// #![feature(extern_types)]

/// Vendor ID for Leap Motion. Leap Motion (now Ultraleap) camera devices contain this in their USB device ID string.
pub const VENDOR_ID__LEAP_MOTION: &'static str = "VID_2936";
/// Product ID for the Rigel, AKA the SIR 170. Rigel / SIR 170 devices contain this in their USB device ID string.
pub const PRODUCT_ID__RIGEL     : &'static str = "PID_1202";

#[cfg(test)]
mod tests;
