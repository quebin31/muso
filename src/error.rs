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

use failure::Fail;

#[derive(Debug, PartialEq, Fail)]
pub enum MusoError {
    #[fail(display = "File type not supported!")]
    NotSupported,

    #[fail(display = "Empty vorbis comments!")]
    EmptyComments,

    #[fail(display = "Parent directory is not valid!")]
    BadParent,

    #[fail(display = "Path {} is not valid as root folder!", _0)]
    InvalidRoot(String),

    #[fail(display = "Tag property {} is missing!", _0)]
    MissingTagProperty(String),

    #[fail(display = "Resource {} was not found!", _0)]
    ResourceNotFound(String),
}
