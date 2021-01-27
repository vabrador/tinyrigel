
use std::{fs::{self, File}, os::linux};
use std::os::unix::io::{IntoRawFd, RawFd};

use nix::{sys::stat::SFlag};

use linux_bindgen;

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
fn can_retrieve_rigel_frame() -> Result<(), &'static str> {
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