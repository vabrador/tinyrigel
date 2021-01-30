// grab_frame.rs
// 
// Retrieves a single frame from tinyrigel using get_rigel(), Rigel::set_callback, Rigel::open, and Rigel::close.
//
// The frame is written to disk as 'grab_frame_example_gen_img.png', at the top-level repository folder.

use image;
use std::{sync::mpsc, time::Duration};
use tinyrigel;

fn main() -> tinyrigel::Result<()> {
  // Retrieve the first connected Rigel if there is one.
  let mut rigel = tinyrigel::get_rigel()?;

  // Set up some channels for the main and frame callback threads to communicate.
  // We're only looking for a single frame, and the callback thread will write the frame to disk, so we need the synchronization provided by these channels.
  let (io_permission_tx, io_permission_rx) = mpsc::channel::<u32>();
  let (io_done_tx, io_done_rx) = mpsc::channel::<u32>();

  // Set the frame callback handler for the Rigel.
  rigel.set_callback(|frame: &[u8]| {
    // Get permission from the main thread to write the frame to disk.
    // Timeout after 100ms to wait for the next frame callback.
    let io_permission = io_permission_rx.recv_timeout(Duration::from_millis(100));
    if io_permission.is_err() { return; }

    // Write the frame to disk.
    let mut img = image::DynamicImage::new_luma8(384 * 2, 384);
    let img_luma8 = img.as_mut_luma8().unwrap();
    img_luma8.copy_from_slice(frame);
    img.save("grab_frame_example_gen_img.png").unwrap();
    println!("[Frame] Saved frame data to test.png.");

    // Send the done signal to the main thread, which could fail if the process halts ot the main thread otherwise hangs up the channel unexpectedly, in which case we just exit.
    let io_done_sent = io_done_tx.send(0u32);
    if io_done_sent.is_err() { return; }
  });

  // Initiate capture.
  rigel.open()?;

  // Give permission to the callback thread to write to disk. (Sending does not block.)
  let gave_permission = io_permission_tx.send(0u32);
  if gave_permission.is_err() { return Err(tinyrigel::Error::new("Unexpectedly failed to give permission to the frame callback thread.".to_string())); }

  // Wait for the callback thread to report the disk write is complete, or fail if we don't hear back from the callback thread in time.
  let io_done = io_done_rx.recv_timeout(Duration::from_millis(500));
  if io_done.is_err() {
    let err_details = io_done.err().unwrap().to_string();
    return Err(tinyrigel::Error::new(format!("Failed to receive io_done signal from the frame callback thread. Either the frame callback was not invoked in time, or the disk write took too long. Receiver channel error was: {}", err_details)));
  }

  // Close the device.
  rigel.close()?;

  Ok(())
}
