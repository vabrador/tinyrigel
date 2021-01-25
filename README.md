# tinyrigel #

It's a tiny Rust library for retrieving Rigel images on Windows, Linux, and macOS. (And maybe iOS? And heck, how bad could Android be?)

Right now, the goal is just to get Rigel images. Maybe one day it'll make sense to apply a calibration to the image too, or maybe not!

It should also be relatively trivial to support an LMC as well as just the Rigel, since it's just an interleaved-frame interpretation of the same 8-bit grayscale pixel format.

- Might also be interesting to make it easy to query and switch over more than one connected Leap device, but one thing at a time.

To test:
```sh
# --nocapture is used to allow tests to print to stdout, to e.g. list devices.
cargo test -- --nocapture
```

## Per-Platform Notes ##

### Windows backend ###

- https://kennykerr.ca/2020/06/09/improving-the-ide-for-rust-winrt/
- https://github.com/microsoft/windows-rs

- Thanks to windows-rs it looks like the Windows backend will be able to stay reasonably clean.

### macOS (and iOS?) backend ###

- https://github.com/SSheldon/rust-objc/issues/12
- https://github.com/ndarilek/tts-rs/blob/master/src/backends/av_foundation.rs
- https://github.com/SSheldon/rust-objc

- macOS TODOs:
  - Need to organize the Xcode reference back into a tidy single cmdl project
  - Need to test the macOS integrate for leaks -- extremely messy, raw objc interop happening in the macOS backend currently -- I don't trust it at all!

### Linux backend ###

More details to follow. Some scattered notes for now:

- leapuvc's C example is actually a very raw posix + v4l2 + SDL example, so it's a good Linux reference:
  - https://github.com/leapmotion/rawviewer/blob/ff68600a19b51187c15cb010c36b73d801d082e8/v4l2sdl.c#L53

- Building with Ubuntu 20.02

- Attempting to use bindgen with v4l2. This will at least require the bindgen requirements on Linux for any Linux builders:
  - https://rust-lang.github.io/rust-bindgen/requirements.html#debian-based-linuxes

## Licensing

The goal is to release this project into the public domain via Unlicense, however, it's unclear how feasible that is given licenses in the project's dependencies. To be reviewed:

- windows-rs license: ???
- objc license: MIT (is this effectively infectious?)
- v4l2 license: ???
