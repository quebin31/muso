// Copyright (C) 2020 Kevin Dc
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

#[cfg(feature = "standalone")]
use std::{fs::File, io::Write};

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{MusoError, Result};

#[inline]
pub fn default_config_path() -> PathBuf {
    dirs::config_dir().unwrap().join("muso/config.toml")
}

#[inline]
pub fn default_service_path() -> PathBuf {
    dirs::config_dir()
        .unwrap()
        .join("systemd/muso/muso.service")
}

pub fn maybe_create_dir(path: impl AsRef<Path>) -> std::io::Result<()> {
    match fs::create_dir_all(path) {
        Err(e) => match e.kind() {
            std::io::ErrorKind::AlreadyExists => Ok(()),
            _ => Err(e),
        },
        Ok(_) => Ok(()),
    }
}

pub fn is_empty_dir(path: impl AsRef<Path>) -> Result<bool> {
    if !path.as_ref().is_dir() {
        Ok(false)
    } else {
        Ok(fs::read_dir(path)?.count() == 0)
    }
}

pub enum Resource {
    Config,
    Service,
}

pub fn generate_resource(res: Resource) -> Result<()> {
    let name = match res {
        Resource::Config => "config",
        Resource::Service => "service",
    };

    let dest = match res {
        Resource::Config => default_config_path(),
        Resource::Service => default_service_path(),
    };

    log::info!("Generating {} file", name);

    cfg_if::cfg_if! {
        if #[cfg(feature = "standalone")] {
            log::info!("Writing {} file", name);

            maybe_create_dir(dest.parent().ok_or(MusoError::InvalidParent {
                child: dest.to_string_lossy().into(),
            })?)?;

            let mut file = File::create(&dest)?;
            let contents = match res {
                Resource::Config => include_str!("../share/config.toml"),
                Resource::Service => include_str!("../share/muso.service"),
            };

            write!(file, "{}", contents)?;

            log::info!("Successfully written to: \"{}\"", dest.to_string_lossy());
        } else {
            let shared = match res {
                Resource::Config => Path::new("/usr/share/muso/muso.service"),
                Resource::Service => Path::new("/usr/share/muso/config.toml"),
            };

            if !shared.exists() {
                return Err(MusoError::ResourceNotFound {
                    path: shared.to_string_lossy().into(),
                }.into());
            } else {
                log::info!("Copying {} file from shared assets", name);

                maybe_create_dir(dest.parent().ok_or(MusoError::InvalidParent {
                    child: dest.to_string_lossy().into(),
                })?)?;
                fs::copy(shared, &dest)?;

                log::info! {
                    "Successfully copied to: \"{}\"",
                    dest.to_string_lossy()
                };
            }
        }
    }

    log::info!("Successfully generated {} file", name);
    Ok(())
}
