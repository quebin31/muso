use std::error::Error as StdError;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    NotSupported,
    EmptyComments,
    CantInferMime,
    BadParent,
    CantGetConfig,
    MissingTagProperty(String),
    MissingConfigProperty(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::EmptyComments => write!(f, "Empty vorbis comments!"),
            Error::NotSupported => write!(f, "File type not supported!"),
            Error::CantInferMime => write!(f, "Cannot infer mime from file!"),
            Error::BadParent => write!(f, "Parent directory is invalid!"),
            Error::CantGetConfig => write!(f, "Cannot get config directory!"),
            Error::MissingTagProperty(prop) => {
                write!(f, "Property \'{}\' in tags is missing!", prop)
            }
            Error::MissingConfigProperty(prop) => {
                write!(f, "Property \'{}\' in config is missing!", prop)
            }
        }
    }
}

impl StdError for Error {}
