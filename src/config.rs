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

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::error::{MusoError, Result};
use crate::utils;

#[derive(Debug, Clone, Deserialize)]
pub struct WatchConfig {
    pub every: Option<u64>,
    pub libraries: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LibraryConfig {
    pub format: String,
    pub folders: Vec<String>,
    #[serde(rename = "exfat-compat")]
    pub exfat_compat: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub watch: WatchConfig,
    pub libraries: HashMap<String, LibraryConfig>,
}

impl Config {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let default = utils::default_config_path();
        let path = path.as_ref();

        if !path.exists() {
            if path == default {
                utils::generate_resource(utils::Resource::Config)?;
            } else {
                return Err(MusoError::InvalidConfig {
                    path: path.to_string_lossy().into(),
                    reason: "not found".into(),
                }
                .into());
            }
        }

        let contents = fs::read_to_string(path)?;
        let mut config: Self = toml::from_str(&contents).map_err(|e| MusoError::InvalidConfig {
            path: path.to_string_lossy().into(),
            reason: e.to_string(),
        })?;

        config.sanitize_paths();
        Ok(config)
    }

    pub fn sanitize_paths(&mut self) {
        for (name, library) in &mut self.libraries {
            let mut sanitized: Vec<String> = Vec::new();

            for folder in &library.folders {
                match shellexpand::full(&folder) {
                    Ok(full) => {
                        let path = Path::new(full.as_ref());
                        if path.exists() && path.is_absolute() {
                            sanitized.push(full.as_ref().into());
                        } else {
                            log::warn! {
                                "Library \"{}\" contains an invalid path: \"{}\"",
                                name,
                                full
                            };
                        }
                    }

                    Err(e) => {
                        log::warn!("Library \"{}\" contains an invalid path: {}", name, e);
                    }
                }
            }

            library.folders = sanitized;
        }
    }

    pub fn search_format_for(&self, path: impl AsRef<Path>) -> Option<&str> {
        for library in self.libraries.values() {
            for folder in &library.folders {
                if Path::new(&folder) == path.as_ref() {
                    return Some(&library.format);
                }
            }
        }

        None
    }
}
