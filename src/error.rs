use derive_more::From;
use std::fmt::Formatter;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, From)]
pub enum Error {
    Core(String),

    #[from]
    Io(std::io::Error),

    #[from]
    DB(rusqlite::Error),
}

impl Error {
    pub fn core(value: impl std::fmt::Display) -> Self {
        Self::Core(value.to_string())
    }
}

impl From<&str> for Error {
    fn from(value: &str) -> Self {
        Self::Core(value.to_string())
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter) -> core::result::Result<(), core::fmt::Error> {
        match self {
            Error::Core(s) => write!(f, "{}", s),
            Error::Io(e) => e.fmt(f),
            Error::DB(e) => e.fmt(f),
        }
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(value: std::string::FromUtf8Error) -> Self {
        Self::Core(value.to_string())
    }
}

impl From<dialoguer::Error> for Error {
    fn from(value: dialoguer::Error) -> Self {
        Self::Core(value.to_string())
    }
}

impl std::error::Error for Error {}
