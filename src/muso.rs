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

use std::collections::HashMap;
use std::env::current_dir;
use std::error::Error;
use std::fs::File;
use std::fs::{copy, create_dir_all, read_dir, rename};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use clap::ArgMatches;
use config::{self, Config};
use dirs;
use log::{error, info, warn};
use notify::{self, DebouncedEvent, RecursiveMode, Watcher};
use rayon::prelude::*;
use shellexpand;

use crate::error::MusoError;
use crate::metadata;

#[derive(Debug)]
pub struct Muso {
    config: Config,
    path: PathBuf,
    format: String,
    watch: bool,
    dryrun: bool,
    recursive: bool,
    exfat_compat: bool,
}

impl Muso {
    pub fn run(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
        let config_path = matches.value_of("config").map_or(
            format! {
                "{}/muso/config.toml",
                dirs::config_dir().unwrap().to_string_lossy()
            },
            |v| v.to_owned(),
        );

        let config_path = Path::new(&config_path);
        if !config_path.exists() {
            let shared_config = Path::new("/usr/share/muso/config.toml");
            let config_dir = dirs::config_dir().unwrap().join("muso");
            if !shared_config.exists() {
                maybe_create_dir(config_dir)?;
                let mut file = File::create(&config_path)?;
                write!(file, "{}", include_str!("../share/config.toml"))?;
            } else if config_path.starts_with(config_dir.to_string_lossy().as_ref()) {
                info!("Copying config from shared assets");
                maybe_create_dir(config_dir)?;
                copy(shared_config, config_path)?;
            } else {
                return Err(
                    MusoError::ResourceNotFound(config_path.to_string_lossy().to_string()).into(),
                );
            }
        }

        let mut config = Config::default();
        config.merge(config::File::new(
            &config_path.to_string_lossy(),
            config::FileFormat::Toml,
        ))?;
        sanitize_paths(&mut config)?;

        let path = matches
            .value_of("path")
            .map_or(current_dir()?, |p| p.into())
            .canonicalize()?
            .to_string_lossy()
            .to_string();

        let format = matches
            .value_of("format")
            .map_or(search_format_for(&config, &path), |f| Some(f.to_owned()))
            .unwrap_or_else(|| "{artist}/{album}/{track} - {title}.{ext}".to_owned());

        let watch = matches.is_present("watch");
        let dryrun = matches.is_present("dryrun");
        let recursive = matches.is_present("recursive");
        let exfat_compat = matches.is_present("exfatcompat");
        let copy_service = matches.is_present("copyservice");

        if copy_service {
            let shared_service = Path::new("/usr/share/muso/muso.service");

            let systemd_path = format! {
                "{}/systemd/user/muso.service",
                dirs::config_dir().unwrap().to_string_lossy()
            };

            if !shared_service.exists() {
                let mut file = File::create(&systemd_path)?;
                write!(file, "{}", include_str!("../share/muso.service"))?;
            } else {
                info!("Copying service file from shared assets");
                copy(shared_service, systemd_path)?;
            }

            return Ok(());
        }

        let mut muso = Self {
            config,
            path: path.into(),
            format,
            watch,
            dryrun,
            recursive: recursive || watch,
            exfat_compat,
        };

        muso.run_inner()
    }

    fn run_inner(&mut self) -> Result<(), Box<dyn Error>> {
        if self.watch {
            let libraries = self.config.get_array("watch.libraries").unwrap_or_default();

            if libraries.is_empty() {
                error!("No directories to watch!");
                return Ok(());
            }

            let (tx, rx) = mpsc::channel();
            let mut watcher = notify::watcher(
                tx,
                Duration::from_secs(self.config.get_int("watch.every").map_or(1u64, |t| {
                    if t < 0 {
                        1
                    } else {
                        t as u64
                    }
                })),
            )?;

            let mut related_library = HashMap::new();
            for library in libraries {
                let library = library.into_str()?;
                let folders = self
                    .config
                    .get_array(&format!("libraries.{}.folders", library))?;

                for folder in folders {
                    let folder = folder.into_str()?;
                    related_library.insert(folder.clone(), library.clone());
                    watcher.watch(&folder, RecursiveMode::Recursive)?;
                }
            }

            self.watch_loop(rx, related_library)
        } else if self.path.is_dir() {
            self.sort_folder(&self.path, &self.path)
                .map(|(success, total)| {
                    info!("Done: {} successful out of {}", success, total);
                })
                .map_err(|e| {
                    error!("{}", e);
                    e
                })
        } else {
            Err(MusoError::InvalidRoot(self.path.to_string_lossy().to_string()).into())
        }
    }

    fn watch_loop(
        &mut self,
        rx: mpsc::Receiver<DebouncedEvent>,
        related_library: HashMap<String, String>,
    ) -> Result<(), Box<dyn Error>> {
        loop {
            match rx.recv() {
                Ok(event) => match event {
                    DebouncedEvent::Create(path)
                    | DebouncedEvent::Write(path)
                    | DebouncedEvent::Rename(_, path) => {
                        for ancestor in path.ancestors() {
                            let ancestor = ancestor.to_string_lossy();
                            if !related_library.contains_key(ancestor.as_ref()) {
                                continue;
                            }

                            let library = Some(related_library[ancestor.as_ref()].as_str());
                            if let Some(library) = library {
                                self.format = self
                                    .config
                                    .get_str(&format!("libraries.{}.format", library))
                                    .ok()
                                    .unwrap_or_else(|| self.format.clone());

                                self.exfat_compat = self
                                    .config
                                    .get_bool(&format!("libraries.{}.exfat-compat", library))
                                    .ok()
                                    .unwrap_or_else(|| self.exfat_compat);
                            }

                            if path.is_dir() {
                                match self.sort_folder(Path::new(ancestor.as_ref()), &path) {
                                    Ok((success, total)) => {
                                        info!("Done: {} successful out of {}", success, total)
                                    }
                                    Err(e) => error!("{}", e),
                                }
                            } else {
                                match self.sort_file(Path::new(ancestor.as_ref()), &path) {
                                    Ok(()) => info!("Done: 1 successful out of 1"),
                                    Err(e) => error!("{}", e),
                                }
                            }
                        }
                    }

                    _ => {}
                },
                Err(e) => error!("{}", e),
            }
        }
    }

    fn sort_folder(&self, root: &Path, folder: &Path) -> Result<(usize, usize), Box<dyn Error>> {
        let results = read_dir(folder)?
            .par_bridge()
            .map(|entry| -> (usize, usize) {
                let entry = entry.expect("Cannot get entry!");
                let file_type = entry.file_type().expect("Cannot get file type!");

                if file_type.is_dir() && self.recursive {
                    match self.sort_folder(root, &entry.path()) {
                        Ok(result) => result,
                        Err(e) => {
                            error!("{}", e);
                            (0, 0)
                        }
                    }
                } else if file_type.is_file() {
                    match self.sort_file(root.as_ref(), &entry.path()) {
                        Ok(_) => (1, 1),
                        Err(e) => {
                            error!("{}", e);
                            (0, 1)
                        }
                    }
                } else {
                    (0, 0)
                }
            });

        let (success, total) = results
            .fold(
                || (0, 0),
                |(success_t, total_t), (success, total)| (success_t + success, total + total_t),
            )
            .reduce(
                || (0, 0),
                |(success_t, total_t), (success, total)| (success_t + success, total + total_t),
            );

        Ok((success, total))
    }

    fn sort_file(&self, root: &Path, file: &Path) -> Result<(), Box<dyn Error>> {
        let metadata = metadata::Metadata::from_path(file)?;
        let new_path = metadata.build_path(&self.format, self.exfat_compat)?;

        if self.dryrun {
            info!("Dry run on: \'{}\'", file.to_string_lossy());
            info!("Item created: \'{}\'", new_path);
        } else {
            info!("Working on: \'{}\'", file.to_string_lossy());
            info!("Item created: \'{}\'", new_path);

            let path = Path::new(&new_path);
            maybe_create_dir(root.join(&path.parent().ok_or(MusoError::BadParent)?))?;
            rename(file, root.join(&path))?;
        }

        Ok(())
    }
}

fn maybe_create_dir(path: impl AsRef<Path>) -> io::Result<()> {
    if let Err(e) = create_dir_all(path) {
        match e.kind() {
            io::ErrorKind::AlreadyExists => Ok(()),
            _ => Err(e),
        }
    } else {
        Ok(())
    }
}

fn sanitize_paths(config: &mut Config) -> Result<(), Box<dyn Error>> {
    let libraries = config.get_table("libraries")?;

    for (library, table) in libraries {
        let table = table.into_table()?;
        let folders = table.get("folders").unwrap().clone().into_array()?;

        let mut sanitized: Vec<config::Value> = Vec::new();
        for folder in folders {
            let folder = folder.clone().into_str()?;

            match shellexpand::full(&folder) {
                Ok(full) => {
                    let path = Path::new(full.as_ref());
                    if path.is_absolute() && path.exists() {
                        sanitized.push(full.as_ref().into());
                    }
                }
                Err(e) => warn!("{}", e),
            }
        }

        config.set(&format!("libraries.{}.folders", library), sanitized)?;
    }

    Ok(())
}

fn search_format_for(config: &Config, path: impl AsRef<Path>) -> Option<String> {
    let libraries = config.get_table("libraries").ok()?;

    for (_, table) in libraries {
        let table = table.into_table().ok()?;
        let folders = table.get("folders")?.clone().into_array().ok()?;

        for folder in folders {
            let folder = folder.into_str().ok()?;

            if Path::new(&folder) == path.as_ref() {
                let format = table.get("format")?.clone().into_str().ok()?;
                return Some(format);
            }
        }
    }

    None
}
