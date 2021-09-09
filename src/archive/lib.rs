// lib.rs - tinyrigel

mod core;
pub use crate::core::*;

mod rigel;
pub use rigel::*;

// Tests
// ---

#[cfg(test)]
mod tests;
