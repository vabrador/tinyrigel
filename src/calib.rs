
/// The latest calibration version identified by platform/develop as of 2021-09-04.
pub const CURRENT_CALIBRATION_VERSION: u8 = 2;
/// The expected byte offset in the bytes of a SerializedCalibration for the checksum field. 
pub const CURRENT_CALIB_VER_CHECKSUM_OFFSET: u32 = 152;

/// Ported from platform/SerializedCalibration.h:72
/// This SerializedCalibration struct should be byte-identical to the platform Cpp one.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SerializedCalibration {
  /// Must be `['C', 'A']`. Identifier header for serialized calibrations.
  signature: [ascii::AsciiChar; 2],

  /// Single-byte version field
  version: u8,

  /// Score of calibration
  score: u8,

  /// Timestamp when this calibration was generated.
  /// 
  /// The value that platform writes when generating timestamps is implementation-dependent. However, the implementations that would only ever execute calibration-writing code would interpret time_t as i64. Platform's code for turning this u32 into a time_t (i64) we are assuming naively pads out the remaining 32 bits since the unsigned 32-bit value fits bitwise into the first 32 bits of an i64 without any e.g. sign bit issues.
  ///
  /// We will also assume this value corresponds to seconds from the UNIX epoch. Apparently the format is actually undefined, but is "usually" seconds from the epoch.
  /// https://en.cppreference.com/w/cpp/chrono/c/time_t
  timestamp: u32,

  /// Other camera parameters - Baseline
  baseline: f32,

  /// Other camera parameters - Q2init
  q2_init: f32,

  /// Two single-camera calibrations, each with intrinsics and extrinsics.
  calibration: [CameraCalibration; 2],

  /// Final field is a checksum. Platform implements this literally as the (naively overflowing?) sum of each u32 before the checksum.
  ///
  /// Note that there apparently is an allowance made in checksum validation that allows the last byte of the checksum to be 0 regardless of the expected checksum, due to some unknown issue with corruption on that platform.
  checksum: u32
}

/// CameraCalibration - see platform/SerializedCalibration.h:55
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CameraCalibration {
  intrinsics: IntrinsicCalibration,
  extrinsics: ExtrinsicCalibation
}

/// See platform/SerializedCalibration.h:13
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IntrinsicCalibration {
  f: f32,
  offset: [f32; 2],
  tangential: [f32; 2],
  radial: [f32; 6],
}

// See platform/SerializedCalibration.h:31
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ExtrinsicCalibation {
  f: f32,
  x0: f32,
  y0: f32,
  rotation: [f32; 3]
}

impl SerializedCalibration {
  /// Returns a calibration consisting entirely of zeros. Such a calibration is not actually a valid calibration to use and is intended only for e.g. empty buffer allocation.
  pub fn empty_calibration() -> SerializedCalibration {
    let null_calibration: SerializedCalibration = unsafe { std::mem::zeroed() };
    null_calibration
  }
}
