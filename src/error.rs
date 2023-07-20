use std::error::Error;

use ext_php_rs::exception::PhpException;

pub struct SpidroinError {
    pub description: String,
}

impl SpidroinError {
    pub fn new(description: String) -> Self {
        Self { description }
    }

    pub fn from<T: Error>(context: &str, error: &T) -> Self {
        Self {
            description: format!("{context}: {}", error),
        }
    }
}

impl From<SpidroinError> for PhpException {
    fn from(value: SpidroinError) -> PhpException {
        PhpException::default(format!("Spidroin Exception: {}", value.description))
    }
}
