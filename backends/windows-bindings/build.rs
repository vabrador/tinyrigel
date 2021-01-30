fn main() {
  windows::build!(
    windows::devices::enumeration::*
    windows::media::capture::*
    windows::media::media_properties::*
    windows::storage::streams::*
  );
}
