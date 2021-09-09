
use packed_struct::prelude::*;

pub use packed_struct::PackedStruct;

/// Control requests to USB devices each begin with an 8-byte Setup Packet.
#[derive(PackedStruct, Debug, Copy, Clone)]
#[packed_struct(bit_numbering="msb0", endian="lsb", size_bytes="8")]
pub struct SetupPacket {
  /// bmRequestType - Recipient (bits 0-4):
  ///
  /// Device = 0,
  /// Interface = 1,
  /// Endpoint = 2,
  /// Other = 3
  #[packed_field(bits="0..=4", ty="enum")]
  pub bm_request_type_recipient: BmRequestTypeRecipient,

  /// bmRequestType - Type (bits 5-6):
  ///
  /// Standard = 0,
  /// Class = 1,
  /// Vendor = 2,
  /// Other = 3
  #[packed_field(bits="5..=6", ty="enum")]
  pub bm_request_type_type: BmRequestTypeType,

  /// bmRequestType - Data Phase Transfer Direction (bit 7):
  ///
  /// Host to Device = 0,
  /// Device to Host = 1
  #[packed_field(bits="7", ty="enum")]
  pub bm_request_type_direction: BmRequestTypeDataPhaseTransferDirection,

  /// The request code identifying what the request is. Standard requests are common to all USB devices, class requests are common to classes of drivers. Vendor requests are vendor-specific.
  pub b_request: u8,

  /// A value parameter associated with the request.
  pub w_value: u16,

  /// An index or offset parameter associated with the request.
  pub w_index: u16,

  /// If there is a data phase to the request, this is the number of bytes to transfer.
  pub w_length: u16
}

/// USB Setup Packet `bmRequestType` Recipient. Bits 0-4. Values from 4 to 31 are reserved.
///
/// Device = 0,
/// Interface = 1,
/// Endpoint = 2,
/// Other = 3
#[derive(PrimitiveEnum_u8, Clone, Copy, Debug, PartialEq)]
pub enum BmRequestTypeRecipient {
  /// 0b0000
  Device = 0,
  /// 0b0001
  Interface = 1,
  /// 0b0010
  Endpoint = 2,
  /// 0b0011
  Other = 3
  // Values from 4 to 31 are reserved.
}

/// USB Setup Packet `bmRequestType` Type. Bits 5-6.
///
/// Standard = 0,
/// Class = 1,
/// Vendor = 2,
/// Other = 3
#[derive(PrimitiveEnum_u8, Clone, Copy, Debug, PartialEq)]
pub enum BmRequestTypeType {
  /// 0b00
  Standard = 0,
  /// 0b01
  Class = 1,
  /// 0b10
  Vendor = 2,
  /// 0b11
  Other = 3
}

/// USB Setup Packet `bmRequestType` Data Phase Transfer Direction. Bit 7.
///
/// Host to Device = 0,
/// Device to Host = 1
#[derive(PrimitiveEnum_u8, Clone, Copy, Debug, PartialEq)]
pub enum BmRequestTypeDataPhaseTransferDirection {
  /// 0b0
  HostToDevice = 0,
  /// 0b1
  DeviceToHost = 1
}

/// UVC Request Codes defined by spec. They go in the b_request field of the Setup Packet for USB control requests. UVC A.8.
#[derive(PrimitiveEnum_u8, Clone, Copy, Debug, PartialEq)]
pub enum UVCRequestCode {
  RcUndefined       = 0x00,
  SetCur            = 0x01,
  SetCurAll         = 0x11,
  GetCur            = 0x81,
  GetMin            = 0x82,
  GetMax            = 0x83,
  GetRes            = 0x84,
  GetLen            = 0x85,
  GetInfo           = 0x86,
  GetDef            = 0x87,
  GetCurAll         = 0x91,
  GetMinAll         = 0x92,
  GetMaxAll         = 0x93,
  GetResAll         = 0x94,
  GetDefAll         = 0x97
}

// pub trait USBRequestCode {
//   fn F
// }