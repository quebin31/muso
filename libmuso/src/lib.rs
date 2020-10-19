pub mod format;
pub mod metadata;

use std::io;
use thiserror::Error;

/// Custom Result type used broadly used across this library
pub type Result<T> = std::result::Result<T, self::Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("File type not supported!")]
    NotSupported,

    #[error("Empty vorbis comments!")]
    EmptyComments,

    #[error("Parent directory of \"{child}\" is not valid!")]
    InvalidParent { child: String },

    #[error("Path {path} is not valid as root folder!")]
    InvalidRoot { path: String },

    #[error("Tag property {tag} is missing!")]
    MissingTag { tag: String },

    #[cfg(not(feature = "standalone"))]
    #[error("Resource \"{path}\" was not found!")]
    ResourceNotFound { path: String },

    #[error("Invalid config file: \"{path}\" ({reason})")]
    InvalidConfig { path: String, reason: String },

    #[error("Failed to parse format string")]
    FailedToParse,

    #[error("Directory components in format string can't contain optionals")]
    OptionalInDir,

    #[error("File component must have one required placeholder (except from {{ext}})")]
    RequiredInFile,

    #[error("I/O error (source: {source})")]
    IoError {
        #[from]
        source: io::Error,
    },

    #[error("Id3 error (source: {source})")]
    Id3Error {
        #[from]
        source: id3::Error,
    },

    #[error("Metaflac error (source: {source})")]
    MetaflacError {
        #[from]
        source: metaflac::Error,
    },
}
