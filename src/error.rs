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
