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

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::format::ParsedFormat;
use crate::{Error, Result};

#[derive(Debug, Clone, Deserialize)]
pub struct WatchConfig {
    pub every: Option<u64>,
    pub libraries: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LibraryConfig {
    pub format: ParsedFormat,
    pub folders: Vec<PathBuf>,

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
        let path = path.as_ref();
        let contents = fs::read_to_string(path)?;

        let mut config: Self = toml::from_str(&contents).map_err(|e| Error::InvalidConfig {
            reason: e.to_string(),
        })?;

        config.sanitize_folders()?;

        Ok(config)
    }

    fn sanitize_folders(&mut self) -> Result<()> {
        let mut seen_folders = HashSet::new();

        for (name, library) in &mut self.libraries {
            let mut sanitized: Vec<PathBuf> = Vec::new();

            for folder in library.folders.drain(..) {
                let folder = if let Some(folder_str) = folder.as_os_str().to_str() {
                    match shellexpand::full(folder_str) {
                        Ok(full) => Path::new(full.as_ref()).to_path_buf(),

                        Err(e) => {
                            log::warn!(
                                "Library \"{}\" contains an invalid path: {} (ignoring)",
                                name,
                                e
                            );
                            continue;
                        }
                    }
                } else {
                    folder
                };

                if !folder.exists() || !folder.is_absolute() {
                    log::warn!(
                        "Library \"{}\" contains an invalid path: {} (ignoring)",
                        name,
                        folder.display()
                    );
                } else if seen_folders.contains(&folder) {
                    log::error!(
                        "Library \"{}\" contains a repeated folder: {}",
                        name,
                        folder.display()
                    );

                    return Err(Error::InvalidConfig {
                        reason: "Repeated folder path in library".into(),
                    });
                } else {
                    sanitized.push(folder.clone());
                    seen_folders.insert(folder);
                }
            }

            library.folders = sanitized;
        }

        Ok(())
    }

    pub fn search_format(&self, path: impl AsRef<Path>) -> Option<&ParsedFormat> {
        let path = path.as_ref().to_path_buf();
        for library in self.libraries.values() {
            if library.folders.contains(&path) {
                return Some(&library.format);
            }
        }

        None
    }

    pub fn format_of(&self, library: &str) -> Option<&ParsedFormat> {
        self.libraries.get(library).map(|library| &library.format)
    }

    pub fn is_exfat_compat(&self, library: &str) -> bool {
        self.libraries
            .get(library)
            .map(|library| library.exfat_compat)
            .flatten()
            .unwrap_or(false)
    }
}
