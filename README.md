# tinyrigel #

It's a tiny Rust library that interfaces with WinRT using windows-rs to get Rigel images on Windows.

TODO: Make a working Mac and Linux backend too.

To test:
```sh
# --nocapture is used to allow tests to print to stdout, to e.g. list devices.
cargo test -- --nocapture
```

## Useful references ##

### Windows backend ###

- https://kennykerr.ca/2020/06/09/improving-the-ide-for-rust-winrt/
- https://github.com/microsoft/windows-rs

### macOS (and iOS?) backend ###

- https://github.com/SSheldon/rust-objc/issues/12
- https://github.com/ndarilek/tts-rs/blob/master/src/backends/av_foundation.rs
- https://github.com/SSheldon/rust-objc

