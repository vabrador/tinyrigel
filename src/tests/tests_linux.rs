
use std::{fs::{self, File}};
use std::os::unix::io::{IntoRawFd, RawFd};

use nix::{sys::stat::SFlag};

// use linux_bindgen;

use v4l::{Capabilities, context::Node, prelude::*};
use v4l::io::mmap::Stream;
use v4l::io::traits::CaptureStream;
use v4l::video::Capture;

// TODO: Probably no need for this test anymore, DELETEME.
#[test]
fn can_enumerate_video_devices() -> Result<(), &'static str> {
  for entry in fs::read_dir("/dev")
  .expect("Failed to open /dev directory") {
    if entry.is_err() {
      println!("Failed to enumerate /dev entry: {}", entry.err().unwrap());
      continue;
    }
    let entry = entry.unwrap();
    if entry.file_name().to_string_lossy().to_string().contains("video") {
      println!("video entry: {}", entry.file_name().to_string_lossy().to_string());
    }
  }

  Ok(())
}

#[test]
fn can_retrieve_rigel_frame() -> Result<(), String> {
  
  // Enumerate connected devices and find the first valid Rigel Device and device node (/dev/videoX entry)
  let (mut rigel, mut rigel_node) = (None, None);
  for device_node in v4l::context::enum_devices() {
    let device = Device::new(device_node.index());
    if device.is_err() {
      println!("(Error getting Device for index {}, skipping it. Inner error was: {}", device_node.index(), device.err().unwrap().to_string());
      continue;
    }
    let device = device.unwrap();

    if is_device_rigel(&device_node, &device) {
      rigel_node = Some(device_node);
      rigel = Some(device);
      break;
    }
  }
  if rigel.is_none() {
    return Err("No Rigel device found in device enumeration. Is your Rigel plugged in?".to_string());
  }
  let mut rigel = rigel.unwrap();
  let rigel_node = rigel_node.unwrap();
  println!("Found Rigel device at index {}, path {}.", rigel_node.index(), rigel_node.path().to_str().unwrap());

  // Enumerate allowed framesizes and frameintervals? There was a nice way to list allowed framesizes -- possibly through format query

  // Set the Rigel to YUYV, 348x384, 90 FPS.
  // ---
  // - Enumerating framesizes for YUYV format reveals all expected framesizes.
  // - Then, querying frameinterval for YUYV @ 384x384 reveals 90fps.
  // - UNTESTED: Need to set format YUYV, set framesize 384x384, set interval 1/90, and then observe that the resulting active format/framesize/interval all matches as expected.
  let yuyv = v4l::FourCC::new(b"YUYV");
  let req_format = v4l::Format::new(384, 384, yuyv);
  let cap_format = rigel.set_format(&req_format);
  if cap_format.is_err() {
    return Err(format!("Failed to set Rigel capture format to 384x384 @ 90 fps. Inner error was: {}", cap_format.err().unwrap().to_string()));
  }
  let cap_format = cap_format.unwrap();
  if cap_format.width != 384 || cap_format.height != 384 {
    return Err(format!("Failed to set Rigel capture format to 384x384. The resulting capture format was {}x{}.", cap_format.width, cap_format.height));
  }
  let frameintervals = rigel.enum_frameintervals(yuyv, 384, 384);
  if frameintervals.is_err() {
    return Err(format!("Failed to enumerate frameintervals for YUYV @ 384x384."));
  }
  let frameintervals = frameintervals.unwrap();
  for frameinterval in &frameintervals {
    println!("{}", frameinterval.to_string());
  }
  let frameinterval_90fps = frameintervals.iter().find(|fi| -> bool {
      match fi.interval {
        v4l::frameinterval::FrameIntervalEnum::Discrete(frac) => {
          frac.numerator == 1 && frac.denominator == 90
        }
        _ => { false }
      }
  });
  if frameinterval_90fps.is_none() {
    return Err(format!("Failed to find 90fps frameinterval for YUYV @ 384x384."));
  }
  
  let mut stream = Stream::with_buffers(&mut rigel, v4l::buffer::Type::VideoCapture, 4)
    .expect("Failed to create buffer stream.");

  let mut i = 0;
  loop {
    let (buf, meta) = stream.next().unwrap();
    println!("Buffer size: {}; seq: {}; timestamp: {}", buf.len(), meta.sequence, meta.timestamp);

    // Save the image as a PNG as a test.
    use image::GenericImageView;
    let mut img = image::DynamicImage::new_luma8(384 * 2, 384);
    let img_luma8 = img.as_mut_luma8().unwrap();
    img_luma8.copy_from_slice(buf);
    println!("[Frame] Copied image from frame data, {}x{}", img.width(), img.height());
    img.save("test.png").unwrap();
    println!("[Frame] Invoked write to test image");

    i += 1;
    if i == 10 { break; }
  }

  println!("Done.");
  Ok(())
}

fn is_device_rigel(device_node: &v4l::context::Node, device: &Device) -> bool {
  let name = device_node.name();
  if name.is_none() { return false; }

  let name = name.unwrap();
  if !name.contains("Leap Motion") || !name.contains("Rigel") { return false; }
  
  let caps = device.query_caps();
  if caps.is_err() { return false; }

  let caps = caps.unwrap();
  if !caps.capabilities.contains(
    v4l::capability::Flags::VIDEO_CAPTURE |
    v4l::capability::Flags::STREAMING
  ) {
    return false;
  }

  true
}

// TODO: Probably standardize this as can_enumerate? (Replacing the one at the top)
#[test]
fn can_query_video_devices() -> Result<(), &'static str> {
  
  for device_node in v4l::context::enum_devices() {
    let device_name = device_node.name();
    if device_name.is_none() {
      println!("(Unable to get device {} name; skipping.)", device_node.index());
      continue;
    }
    let device_name = device_name.unwrap();

    let device_idx = device_node.index();
    let device_path = device_node.path().to_str().unwrap();
    println!("\nDevice {} ({}): {}", device_idx, device_path, device_name);

    let device = Device::new(device_idx);
    if device.is_err() {
      println!("Unable to create Device from {} ({}), skipping.", device_idx, device_path);
      continue;
    }
    let device = device.unwrap();

    // Device Capabilities
    // ---

    let caps = device.query_caps();
    if caps.is_err() {
      println!("Unable to get device {} capabilities. Continuing.", device_idx);
      continue;
    }
    let caps = caps.unwrap();
    println!("\nDevice {} capabilities:\n{}", device_idx, caps.to_string());
    if !caps.capabilities.contains(v4l::capability::Flags::VIDEO_CAPTURE) {
      println!("Device {} does not have the VIDEO_CAPTURE capability. Continuing.", device_idx);
      continue;
    }

    // Device Controls
    // ---

    let controls = device.query_controls();
    if controls.is_err() {
      println!("Unable to get device {} controls.", device_idx);
    } else {
      let controls = controls.unwrap();

      println!("## {} Controls ##", device_name);
      let mut max_name_len = 0;
      for control in &controls {
        if control.name.len() > max_name_len {
          max_name_len = control.name.len();
        }
      }
      for control in controls {
        println!(
          "{:indent$} : [{}, {}]",
          control.name,
          control.minimum,
          control.maximum,
          indent = max_name_len
        );
      }
    }

    // Formats
    // ---
    
    let formats = device.enum_formats();
    if formats.is_err() {
      println!("Unable to enumerate formats for {}.", device_name);
    } else {
      let formats = formats.unwrap();
      for (idx, format) in formats.iter().enumerate() {
        println!("Available format {} for device {}:", idx, device_name);
        println!("{}", format.to_string());
      }
    }

    // Framesizes
    // ---

    let framesizes = device.enum_framesizes(v4l::FourCC::new(b"YUYV"));
    if framesizes.is_err() {
      println!("Unable to enumerate framesizes for FourCC 'YUYV' @ 384x384.");
    } else {
      let framesizes = framesizes.unwrap();
      for (idx, framesize) in framesizes.iter().enumerate() {
        println!("(YUYV 384x384) Available framesize {}: {}", idx, framesize.to_string());
      }
    }

    let frameintervals = device.enum_frameintervals(v4l::FourCC::new(b"YUYV"), 384, 384);
    if frameintervals.is_err() {
      println!("Unable to enumerate frameintervals for FourCC 'YUYV' @ 384x384.");
    } else {
      let frameintervals = frameintervals.unwrap();
      for (idx, frameinterval) in frameintervals.iter().enumerate() {
        println!("(YUYV 384x384) Available frameinterval {}: {}", idx, frameinterval.to_string());
      }
    }

    let params = device.params();
    if params.is_err() {
      println!("Unable to enumerate params for {}.", device_name);
    } else {
      let params = params.unwrap();
      println!("{} params: {}", device_name, params.to_string());
      // for (idx, param) in .enumerate() {
      //   println!("Param {}: {}", idx, param.to_string());
      // }
    }
  }

  Ok(())
}

#[test]
fn old_bak_retrieve_rigel_frame() -> Result<(), &'static str> {
  for entry in fs::read_dir("/dev")
  .expect("Failed to open /dev directory") {
    if entry.is_err() {
      println!("Failed to enumerate /dev entry: {}", entry.err().unwrap());
      continue;
    }
    let entry = entry.unwrap();
    let entry_name = entry.file_name().to_string_lossy().to_string();
    if !entry_name.contains("video") {
      continue;
    }
    println!("video device entry: {}", entry_name);

    // Quick check: We expect the device files not to actually be files, or directories.
    let entry_type = entry.file_type().expect(format!("Error getting {} file type", entry.file_name().to_string_lossy()).as_str());
    if entry_type.is_file() {
      println!("Unexpectedly, entry_type for {} was a file, skipping.", entry_name);
      continue;
    } else if entry_type.is_dir() {
      println!("Unexpectly, entry_type for {} was a dir, skipping.", entry_name);
      continue;
    }
    let device_file = File::open(entry.path()).expect("Error getting file for device");

    // Get raw file descriptor and stat output for the device to check whether it's actually a device file.
    let device_fd: RawFd = device_file.into_raw_fd();
    let device_stat = nix::sys::stat::fstat(device_fd).expect("Failed to get fstat");
    if device_stat.st_mode & SFlag::S_IFCHR.bits() == 0 {
      println!("Device {} is not a character device file.", entry_name);
      continue;
    }

    println!("Device {} is a special character device. Here's its stat output:\n{:?}", entry_name, device_stat);
    
    // Ref: Query device capabilities via v4l2
    // ("video_fd" refers to the video device "file descriptor".)
    // ---
    // errno = v4l2_querycap(video_fd, &cap);
    // if (!(cap.capabilities & V4L2_CAP_VIDEO_CAPTURE)) {
    //   printf("V4L2_CAP_VIDEO_CAPTURE not supported\n");
    //   goto main_exit;
    // }
    // (now we have device capabilities in "cap"...)
    // RELEVANT LOCAL:
    // struct v4l2_capability cap;
    //

    // linux_bindgen::VIDIOC_QUE

    // let foo = nix::sys::ioctl::ioctl_read! {
    //   /// See:
    //   /// https://github.com/torvalds/linux/blob/2ab38c17aac10bf55ab3efde4c4db3893d8691d2/include/uapi/linux/videodev2.h#L430
    //   /// As retrieved by:
    //   /// https://github.com/leapmotion/rawviewer/blob/ff68600a19b51187c15cb010c36b73d801d082e8/v4l2sdl.c#L65
    // }

    // int v4l2_querycap(int fd, struct v4l2_capability *cap)
    // {
    //   if (-1 == ioctl(fd, VIDIOC_QUERYCAP, cap))
    //     printf("VIDIOC_QUERYCAP failure");
    //   printf("driver:%s\n", cap->driver);
    //   printf("card:%s\n", cap->card);
    //   printf("bus:%s\n", cap->bus_info);
    //   printf("version:0x%x\n", cap->version);
    //   printf("capabilities:0x%x\n", cap->capabilities);
    //   printf("device_caps:0x%x\n", cap->device_caps);
    //   return 0;
    // }

    // Ref: Use capabilities to check whether we can perform streaming capture.
    // ---
    // if (!(cap.capabilities & V4L2_CAP_STREAMING)) {
    //     printf("V4L2_CAP_STREAMING not supported\n");
    //    goto main_exit;
    //  }
    // (now we know the device supports v4l2 streaming...)
    // RELEVANT LOCAL:
    // struct v4l2_capability cap;

    // Ref: Attempt to specify the format of the capture for the device.
    // ---
    // errno = v4l2_sfmt(video_fd, width / 2, height, pixel_format);
    // if (errno) {
    //   printf("VIDIOC_S_FMT failure\n");
    //   goto main_exit;
    // }
    // (no idea what this block is checking but I guess it's to ... set.. format?)
    // yeah -- and note: global: uint32_t pixel_format = V4L2_PIX_FMT_YUYV;
    // //printf("VIDIOC_S_FMT success\n");

    // NOTE: MOST LIKELY, HERE ^^^ IS WHERE WE WILL BE ABLE TO DETERMINE IF THE DEVICE IS A RIGEL -- BY QUERYING SOME PROPERTIES OR ID.

    // Ref: Attempt to specify the memory map for video buffer data (to READ from callback function? or WRITE to? Unclear right now!)
    // ---
    // errno = v4l2_mmap(video_fd, video_buffers, &video_buf_len);
    // if (errno) {
    //   printf("v4l2 mmap failure\n");
    //   goto main_exit;
    // }
    // RELEVANT GLOBALS:
    // int video_buf_len;
    // void *video_buffers[BUF_NUM];
    // WHAT SETS THESE?

    // Ref: Set handle video device, callback func, begin stream (pass in handle struct).
    // ---
    // handle.fd = video_fd;
    // handle.call_back = callback_func;
    // errno = v4l2_streamon(video_fd, &handle);
    // RELEVANT LOCAL:
    // struct cb_handle handle;

    // (SDL event loop occurs here.)

    // Cleanup.
    // ===

    // REF: Turn off streaming.
    // ---
    // errno = v4l2_streamoff(video_fd);

    // REF: Unmap the memory, which may fail.
    // errno = v4l2_munmap(video_buffers, video_buf_len);
    // if (errno) {
    //   printf("v4l2 munmap failure\n");
    //   goto main_exit;
    // }

    // NO MATTER WHAT -- CLEANUP
    // ===

    // REF: Close the device.
    // ---
    // errno = v4l2_close(video_fd);
    // if (errno) {
    //   printf("close error %d\n", errno);
    //   exit(1);
    // }

  }
  // Done enumerating devices.

  Ok(())
}