use std::error;
use std::fmt;
extern crate bincode;

pub type Result<T> = std::result::Result<T, TgcError>;

#[derive(Debug)]
pub enum TgcError {
    ThreadError(std::boxed::Box<dyn std::any::Any + std::marker::Send>),
    ChanReceiveError(std::sync::mpsc::RecvError),
    IOError(std::io::Error),
    SerializeError(std::boxed::Box<bincode::ErrorKind>),
}

impl fmt::Display for TgcError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TgcError::ThreadError(_) => write!(f, "thread error"),
            TgcError::ChanReceiveError(ref e) => write!(f, "chan receive error: {}", e),
            TgcError::IOError(ref e) => write!(f, "io error: {}", e),
            TgcError::SerializeError(ref e) => write!(f, "serialize error: {}", e),
        }
    }
}

impl error::Error for TgcError {
    fn description(&self) -> &str {
        match self {
            TgcError::ThreadError(_) => "thread error", // todo: better error message
            TgcError::ChanReceiveError(e) => e.description(),
            TgcError::IOError(e) => e.description(),
            TgcError::SerializeError(e) => e.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match self {
            TgcError::ThreadError(_e) => None,
            TgcError::ChanReceiveError(e) => e.cause(),
            TgcError::IOError(e) => e.cause(),
            TgcError::SerializeError(e) => e.cause(),
        }
    }
}

impl From<std::boxed::Box<bincode::ErrorKind>> for TgcError {
    fn from(err: std::boxed::Box<bincode::ErrorKind>) -> TgcError {
        TgcError::SerializeError(err)
    }
}

impl From<std::io::Error> for TgcError {
    fn from(err: std::io::Error) -> TgcError {
        TgcError::IOError(err)
    }
}

impl From<std::boxed::Box<dyn std::any::Any + std::marker::Send>> for TgcError {
    fn from(err: std::boxed::Box<dyn std::any::Any + std::marker::Send>) -> TgcError {
        TgcError::ThreadError(err)
    }
}

impl From<std::sync::mpsc::RecvError> for TgcError {
    fn from(err: std::sync::mpsc::RecvError) -> TgcError {
        TgcError::ChanReceiveError(err)
    }
}
