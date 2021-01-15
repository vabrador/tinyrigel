
// TODO: Also condition out Windows-specific imports for non-Windows... 

#[cfg(not(windows))]
fn main() -> Result<(), &'static str> {
  Err("grab_frame not implemented for non-Windows platforms.")
}

#[cfg(windows)]
fn main() -> Result<(), &'static str> {
  

  Ok(())
}
