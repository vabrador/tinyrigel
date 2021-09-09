
fn main() {
  let leap_vid = tinyrigel::usb_info::LEAP_USB_VID;
  // let rigel_pid = tinyrigel::usb_info::LEAP_USB_PID_RIGEL;

  find_devices(leap_vid).unwrap();
}

fn find_devices(vid: u16) -> rusb::Result<()> {
  let mut found_leap_device = false;
  for device in rusb::DeviceList::new()?.iter() {
    let device_desc = match device.device_descriptor() { Ok(d) => d, Err(_) => continue };
    
    if device_desc.vendor_id() != vid { continue; }
    
    println!(
      "Found a Leap device: {:04x}:{:04x} ({})",
      device_desc.vendor_id(),
      device_desc.product_id(),
      tinyrigel::usb_info::friendly_name_from_usb_id(device_desc.product_id())
    );
    found_leap_device = true;
  }

  if !found_leap_device {
    println!("No Leap devices found in USB enumeration.");
  }

  Ok(())
}
