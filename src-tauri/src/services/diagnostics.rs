use std::fmt;

#[derive(Debug)]
pub struct DiagnosticsError {
  details: String,
}

impl DiagnosticsError {
  pub fn new(details: impl Into<String>) -> Self {
    Self {
      details: details.into(),
    }
  }
}

impl fmt::Display for DiagnosticsError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(formatter, "{}", self.details)
  }
}

impl std::error::Error for DiagnosticsError {}

pub fn collect_diagnostics() -> Result<(), DiagnosticsError> {
  Ok(())
}
