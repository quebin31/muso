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

use std::collections::{HashMap, HashSet};
use std::env::current_dir;
use std::fs::{copy, create_dir_all, read_dir, remove_dir, rename, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use cfg_if::cfg_if;
use clap::ArgMatches;
use config::{self, Config};
use dirs;
use failure::Error;
use log::{error, info, warn};
use notify::{self, DebouncedEvent, RecursiveMode, Watcher};
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
    ignore_paths: HashSet<PathBuf>,
}

impl Muso {
    pub fn run(matches: &ArgMatches) -> Result<(), Error> {
        // Detect copyservice flag early on, this flag will only
        // copy the service, and nothing else.
        if matches.is_present("copyservice") {
            let shared_service = Path::new("/usr/share/muso/muso.service");

            let user_service = dirs::config_dir()
                .unwrap()
                .join("systemd/user/muso.service");

            // Check the existence of service on /usr/share/muso/muso.service
            // If building with standalone feature include contents of
            // muso.service in the binary, otherwise just fail.
            if !shared_service.exists() {
                cfg_if! {
                    if #[cfg(feature = "standalone")] {
                        info!("Writing service file");
                        let mut file = File::create(&user_service)?;
                        write!(file, "{}", include_str!("../share/muso.service"))?;
                        info!("Successfully writed to: \"{}\"", user_service.to_string_lossy());
                    } else {
                        return Err(MusoError::ResourceNotFound(
                            shared_service.to_string_lossy().to_string(),
                        )
                        .into());
                    }
                }
            } else {
                info!("Copying service file from shared assets");
                copy(shared_service, &user_service)?;
                info! {
                    "Successfully copied to: \"{}\"",
                    user_service.to_string_lossy()
                };
            }

            return Ok(());
        }

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
                cfg_if! {
                    if #[cfg(feature = "standalone")] {
                        info!("Writing default config: \"{}\"", config_path.to_string_lossy());

                        maybe_create_dir(config_dir)?;
                        let mut file = File::create(&config_path)?;
                        write!(file, "{}", include_str!("../share/config.toml"))?;
                    } else {
                        return Err(MusoError::ResourceNotFound(
                            shared_config.to_string_lossy().to_string(),
                        )
                        .into());
                    }
                }
            } else if config_path.starts_with(&config_dir) {
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

        let mut muso = Self {
            config,
            path: path.into(),
            format,
            watch,
            dryrun,
            recursive: recursive || watch,
            exfat_compat,
            ignore_paths: Default::default(),
        };

        muso.run_inner()
    }

    fn run_inner(&mut self) -> Result<(), Error> {
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

            info!("Watching libraries...");
            self.watch_loop(rx, related_library)
        } else if self.path.is_dir() {
            let path = self.path.clone();
            self.sort_folder(&path, &path)
                .map(|(success, total)| {
                    info!(
                        "Done: {} successful out of {} ({} failed)",
                        success,
                        total,
                        total - success
                    );
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
    ) -> Result<(), Error> {
        loop {
            let event = rx.recv();
            if let Err(e) = event {
                error!("{}", e);
                continue;
            }

            match event.unwrap() {
                DebouncedEvent::Rescan => {
                    continue;
                }

                DebouncedEvent::Create(path) | DebouncedEvent::Rename(_, path) => {
                    if self.is_ignored(&path) {
                        self.ignore_paths.remove(&path);
                        continue;
                    }

                    if let Some(ancestor) = self.get_ancestor_for(&path, &related_library) {
                        self.set_options_from(&ancestor, &related_library);

                        if path.is_dir() {
                            match self.sort_folder(&ancestor, &path) {
                                Ok((success, total)) => info!(
                                    "Done: {} successful out of {} ({} failed)",
                                    success,
                                    total,
                                    total - success
                                ),
                                Err(e) => error!("{}", e),
                            }
                        } else {
                            match self.sort_file(&ancestor, &path) {
                                Ok(()) => info!("Done: 1 successful out of 1 (0 failed)"),
                                Err(e) => error!("{}", e),
                            }
                        }
                    }
                }

                _ => {}
            }
        }
    }

    fn sort_folder(&mut self, root: &Path, folder: &Path) -> Result<(usize, usize), Error> {
        let results = read_dir(folder)?.map(|entry| -> (usize, usize) {
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

        let (success, total) = results.fold((0, 0), |(success_t, total_t), (success, total)| {
            (success_t + success, total + total_t)
        });

        if dir_is_empty(folder)? {
            info!("Removing empty folder: \"{}\"", folder.to_string_lossy());
            remove_dir(folder)?;
        }

        Ok((success, total))
    }

    fn sort_file(&mut self, root: &Path, file: &Path) -> Result<(), Error> {
        if self.dryrun {
            info!("Dry run on: \"{}\"", file.to_string_lossy());
        } else {
            info!("Working on: \"{}\"", file.to_string_lossy());
        }

        let metadata = metadata::Metadata::from_path(file)?;
        let new_path = metadata.build_path(&self.format, self.exfat_compat)?;

        if self.dryrun {
            info!("Item created: \"{}\"", new_path);
        } else {
            let new_path = root.join(&new_path);
            let new_path_parent = new_path.parent().ok_or(MusoError::BadParent)?;

            maybe_create_dir(new_path_parent)?;
            rename(file, &new_path)?;

            info!("Item created: \"{}\"", new_path.to_string_lossy());

            if self.watch {
                if new_path_parent != root {
                    self.ignore_paths.insert(new_path_parent.to_path_buf());
                }

                self.ignore_paths.insert(new_path.to_path_buf());
            }
        }

        Ok(())
    }

    fn get_ancestor_for(
        &self,
        path: impl AsRef<Path>,
        related_library: &HashMap<String, String>,
    ) -> Option<PathBuf> {
        for ancestor in path.as_ref().ancestors() {
            let ancestor = ancestor.to_string_lossy();
            if !related_library.contains_key(ancestor.as_ref()) {
                continue;
            } else {
                return Some(ancestor.as_ref().into());
            }
        }

        None
    }

    fn set_options_from(
        &mut self,
        ancestor: impl AsRef<Path>,
        related_library: &HashMap<String, String>,
    ) {
        let ancestor = ancestor.as_ref().to_string_lossy();
        let library = &related_library[ancestor.as_ref()];

        let format = self
            .config
            .get_str(&format!("libraries.{}.format", library))
            .ok();

        if let Some(format) = format {
            self.format = format;
        }

        let exfat_compat = self
            .config
            .get_bool(&format!("libraries.{}.exfat-compat", library))
            .ok();

        if let Some(exfat_compat) = exfat_compat {
            self.exfat_compat = exfat_compat;
        }
    }

    fn is_ignored(&self, path: impl AsRef<Path>) -> bool {
        if path.as_ref().is_file() {
            self.ignore_paths.contains(path.as_ref())
        } else {
            for ignored in &self.ignore_paths {
                if !ignored.is_dir() {
                    continue;
                }

                if ignored.starts_with(path.as_ref()) {
                    return true;
                }
            }

            false
        }
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

fn dir_is_empty(path: impl AsRef<Path>) -> Result<bool, Error> {
    if !path.as_ref().is_dir() {
        Ok(false)
    } else {
        Ok(read_dir(path)?.count() == 0)
    }
}

fn sanitize_paths(config: &mut Config) -> Result<(), Error> {
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
