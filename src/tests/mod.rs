// tests/mod.rs

#[cfg(target_os = "windows")]
mod tests_windows;

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod tests_macos;

#[cfg(target_os = "linux")]
mod tests_linux;
