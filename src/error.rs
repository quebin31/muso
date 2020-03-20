// Copyright (C) 2020 kevin
//
// This file is part of muso.
//
// muso is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// muso is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with muso.  If not, see <http://www.gnu.org/licenses/>.

use thiserror::Error;

#[derive(Debug, PartialEq, Error)]
pub enum MusoError {
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
}

pub type Result<T> = std::result::Result<T, anyhow::Error>;
