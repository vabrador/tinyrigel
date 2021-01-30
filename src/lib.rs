// lib.rs - tinyrigel

// Misc
// ---

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
  details: String
}
impl Error {
  pub fn new(details: String) -> Self { Self {
    details: details
  }}

  pub fn to_string(self: &Self) -> String { self.details.clone() }
}

// Rigel
// ---

pub struct Rigel<Cb>
where Cb: Fn(&[u8]) -> ()
{
  callback_fn: Option<Cb>
}

pub fn get_rigel<Cb>() -> Result<Rigel<Cb>>
where Cb: Fn(&[u8]) -> ()
{
  Err(Error::new("get_rigel not yet implemented.".to_string()))
  // Ok(Rigel { callback_fn: None })
}

impl<Cb> Rigel<Cb>
where Cb: Fn(&[u8]) -> ()
{
  pub fn set_callback(&mut self, callback_fn: Cb) {
    self.callback_fn = Some(callback_fn);
  }

  pub fn open(&mut self) -> Result<()> {
    Err(Error::new("open() not yet implemented.".to_string()))
  }

  pub fn close(&mut self) -> Result<()> {
    Err(Error::new("close() not yet implemented.".to_string()))
  }
}

// Tests
// ---

#[cfg(test)]
mod tests;
