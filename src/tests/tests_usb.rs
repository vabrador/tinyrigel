
use packed_struct::{PackedStruct, PackingError};
use crate::usb;

#[test]
fn test_usb_setup_packet_size() -> Result<(), PackingError> {
  let packet = usb::SetupPacket {
    bm_request_type_recipient: usb::BmRequestTypeRecipient::Device,
    bm_request_type_type: usb::BmRequestTypeType::Class,
    bm_request_type_direction: usb::BmRequestTypeDataPhaseTransferDirection::DeviceToHost,
    b_request: 0,
    w_value: 0,
    w_index: 0,
    w_length: 0,
  };

  assert_eq!(packet.pack()?.len(), 8); // Setup packets are 8 bytes long, by spec.

  Ok(())
}
