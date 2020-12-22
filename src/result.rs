use std::fmt;
pub use std::io::Error as IoError;
pub use std::io::ErrorKind;

pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

pub type Result<T> = std::result::Result<T, Error>;

pub fn to_io_error(e: Error) -> IoError {
    return IoError::new(ErrorKind::Other, e);
}

#[derive(Debug)]
pub struct WeldError {
    message: String,
}

impl WeldError {
    pub fn new(message: String) -> WeldError {
        WeldError { message }
    }
}

impl fmt::Display for WeldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WeldError {{ {} }}", self.message)
    }
}

impl std::error::Error for WeldError {}

#[macro_export]
macro_rules! weld_err {
    ($($arg:tt)*) => { Box::new(WeldError::new(format!($($arg)*))) }
}
