
fn main() {
  winrt::build!(
      // windows::devices::*
      windows::devices::enumeration::*
      // windows::media::*
      windows::media::capture::*
  );
}
