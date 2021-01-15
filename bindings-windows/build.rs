
fn main() {
  windows::build!(
    windows::devices::enumeration::*
    windows::media::capture::*
    windows::media::media_properties::*
    windows::storage::streams::*
  );
}

// // Setup example from: 
// windows::build!(
//   dependencies
//       os
//   types
//       windows::system::diagnostics::*
// );

// fn main() {
//   build();
// }
