use std::error::Error;
use std::fmt;

#[derive(Debug, PartialEq)]
pub enum MusoError {
    NotSupported,
    EmptyComments,
    BadParent,
    InvalidConfigPath(String),
    InvalidRoot(String),
    MissingTagProperty(String),
}

impl Error for MusoError {}

impl fmt::Display for MusoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MusoError::EmptyComments => write!(f, "Empty vorbis comments!"),
            MusoError::NotSupported => write!(f, "File type not supported!"),
            MusoError::BadParent => write!(f, "Parent directory is invalid!"),
            MusoError::InvalidConfigPath(path) => {
                write!(f, "Path \'{}\' is not valid for config!", path)
            }
            MusoError::InvalidRoot(root) => write!(f, "\'{}\' as root folder is invalid!", root),
            MusoError::MissingTagProperty(prop) => {
                write!(f, "Property \'{}\' in tags is missing!", prop)
            }
        }
    }
}
