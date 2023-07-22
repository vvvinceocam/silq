use std::error::Error;

use ext_php_rs::exception::PhpException;

pub struct SilqError {
    pub description: String,
}

impl SilqError {
    pub fn new(description: String) -> Self {
        Self { description }
    }

    pub fn from<T: Error>(context: &str, error: &T) -> Self {
        Self {
            description: format!("{context}: {}", error),
        }
    }
}

impl From<SilqError> for PhpException {
    fn from(value: SilqError) -> PhpException {
        PhpException::default(format!("Silq Exception: {}", value.description))
    }
}
