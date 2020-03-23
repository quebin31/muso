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
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::mpsc;
use std::time::Duration;

use clap::ArgMatches;
use notify::{self, DebouncedEvent, RecursiveMode, Watcher};

use crate::args::Args;
use crate::config::Config;
use crate::error::{AnyResult, MusoError};
use crate::format::ParsedFormat;
use crate::metadata;
use crate::utils;

#[derive(Debug, Clone)]
pub struct Muso {
    args: Args,
    config: Config,
    parsed_format: ParsedFormat,
    ignore_paths: HashSet<PathBuf>,
}

impl Muso {
    pub fn from_matches(matches: ArgMatches) -> AnyResult<Self> {
        let config_path = matches
            .value_of("config")
            .map_or(utils::default_config_path().to_string_lossy().into(), |v| {
                v.to_string()
            });

        let config = Config::from_path(config_path)?;
        let args = Args::from_matches(matches, &config)?;
        let parsed_format = ParsedFormat::from_str(&args.format)?;

        Ok(Self {
            args,
            config,
            parsed_format,
            ignore_paths: Default::default(),
        })
    }

    pub fn run(mut self) -> AnyResult<()> {
        if self.args.watch_mode {
            if self.config.libraries.is_empty() {
                log::info!("No directories to watch!");
                return Ok(());
            }

            let (tx, rx) = mpsc::channel();
            let mut watcher = notify::watcher(
                tx,
                Duration::from_secs(self.config.watch.every.unwrap_or(1u64)),
            )?;

            let mut library_for = HashMap::new();
            for (name, library) in &self.config.libraries {
                for folder in &library.folders {
                    library_for.insert(folder.clone(), name.clone());
                    watcher.watch(folder, RecursiveMode::Recursive)?;
                }
            }

            log::info!("Watching libraries");
            self.watch_loop(rx, library_for)
        } else if self.args.working_path.is_dir() {
            let working_path = self.args.working_path.clone();
            self.sort_folder(&working_path, &working_path)
                .map(|(success, total)| {
                    log::info! {
                        "Done: {} successful out of {} ({} failed)",
                        success,
                        total,
                        total - success
                    };
                })
        } else {
            Err(MusoError::InvalidRoot {
                path: self.args.working_path.to_string_lossy().into(),
            }
            .into())
        }
    }

    fn watch_loop(
        &mut self,
        rx: mpsc::Receiver<DebouncedEvent>,
        library_for: HashMap<String, String>,
    ) -> AnyResult<()> {
        loop {
            let event = rx.recv();
            if let Err(e) = event {
                log::error!("{}", e);
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

                    if let Some(ancestor) = self.get_ancestor_for(&path, &library_for) {
                        if let Err(e) = self.set_options_from(&ancestor, &library_for) {
                            log::error!("{}", e);
                            continue;
                        }

                        if path.is_dir() {
                            match self.sort_folder(&ancestor, &path) {
                                Ok((success, total)) => log::info! {
                                    "Done: {} successful out of {} ({} failed)",
                                    success,
                                    total,
                                    total - success
                                },
                                Err(e) => log::error!("{}", e),
                            }
                        } else {
                            match self.sort_file(&ancestor, &path) {
                                Ok(()) => log::info!("Done: 1 successful out of 1 (0 failed)"),
                                Err(e) => log::error!("{}", e),
                            }
                        }
                    }
                }

                _ => {}
            }
        }
    }

    fn sort_folder(&mut self, root: &Path, folder: &Path) -> AnyResult<(usize, usize)> {
        let results = fs::read_dir(folder)?.map(|entry| -> (usize, usize) {
            let entry = entry.expect("Cannot get entry!");
            let file_type = entry.file_type().expect("Cannot get file type!");

            if file_type.is_dir() && self.args.recursive {
                match self.sort_folder(root, &entry.path()) {
                    Ok(result) => result,
                    Err(e) => {
                        log::error!("{}", e);
                        (0, 0)
                    }
                }
            } else if file_type.is_file() {
                match self.sort_file(root.as_ref(), &entry.path()) {
                    Ok(_) => (1, 1),
                    Err(e) => {
                        log::error!("{}", e);
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

        if utils::is_empty_dir(folder)? {
            log::info!("Removing empty folder: \"{}\"", folder.to_string_lossy());
            fs::remove_dir(folder)?;
        }

        Ok((success, total))
    }

    fn sort_file(&mut self, root: &Path, file: &Path) -> AnyResult<()> {
        if self.args.dryrun {
            log::info!("Dry run on: \"{}\"", file.to_string_lossy());
        } else {
            log::info!("Working on: \"{}\"", file.to_string_lossy());
        }

        let metadata = metadata::Metadata::from_path(file)?;
        let new_path = self
            .parsed_format
            .build_path(&metadata, self.args.exfat_compat)?;

        if self.args.dryrun {
            log::info!("Item created: \"{}\"", new_path);
        } else {
            let new_path = root.join(&new_path);
            let new_path_parent = new_path.parent().ok_or(MusoError::InvalidParent {
                child: new_path.to_string_lossy().into(),
            })?;

            utils::maybe_create_dir(new_path_parent)?;
            fs::rename(file, &new_path)?;

            log::info!("Item created: \"{}\"", new_path.to_string_lossy());

            if self.args.watch_mode {
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
        library_for: &HashMap<String, String>,
    ) -> Option<PathBuf> {
        for ancestor in path.as_ref().ancestors() {
            let ancestor = ancestor.to_string_lossy();
            if !library_for.contains_key(ancestor.as_ref()) {
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
        library_for: &HashMap<String, String>,
    ) -> AnyResult<()> {
        let ancestor = ancestor.as_ref().to_string_lossy();
        let library = &library_for[ancestor.as_ref()];

        let format = self.config.libraries[library].format.clone();
        let exfat_compat = self.config.libraries[library].exfat_compat;

        self.args.format = format;
        self.args.exfat_compat = exfat_compat.unwrap_or(self.args.exfat_compat);
        self.parsed_format = ParsedFormat::from_str(&self.args.format)?;

        Ok(())
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
