// core.rs - tinyrigel

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
  details: String
}

impl Error {
  pub fn new(details: String) -> Self { Self { details: details }}
  pub fn to_string(self: &Self) -> String { self.details.clone() }
}
