use std::env::current_dir;
use std::error::Error as StdError;
use std::fs::{create_dir_all, read_dir, rename, DirEntry};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use clap::ArgMatches;
use config::{self, Config};
use dirs;
use notify::{self, RecursiveMode, Watcher};
use rayon::prelude::*;

use crate::error::Error;
use crate::metadata;

#[derive(Debug)]
pub struct Muso {
    config: Config,
}

impl Muso {
    pub fn new(matches: &ArgMatches) -> Result<Self, Box<dyn StdError>> {
        let config_path = matches.value_of("config").map_or(
            format!(
                "{}/muso/config.toml",
                dirs::config_dir()
                    .ok_or(Error::CantGetConfig)?
                    .to_str()
                    .unwrap()
            ),
            |v| v.to_owned(),
        );

        let mut config = Config::default();
        config.merge(config::File::new(&config_path, config::FileFormat::Toml))?;
        expand_paths(&mut config)?;

        let path = matches
            .value_of("path")
            .map_or(current_dir()?, |p| p.into())
            .canonicalize()?
            .to_str()
            .unwrap()
            .to_owned();

        let format = matches
            .value_of("format")
            .map_or(search_format_for(&config, &path), |f| Some(f.to_owned()))
            .unwrap_or_else(|| "{artist}/{album}/{track} - {title}.{ext}".to_owned());

        let dryrun = matches.is_present("dryrun");
        let watch = matches.is_present("watch");

        config.set("path", path)?;
        config.set("format", format)?;
        config.set("dryrun", dryrun)?;
        config.set("watch", watch)?;

        Ok(Self { config })
    }

    pub fn run(&self) -> Result<(), Box<dyn StdError>> {
        if self.config.get_bool("watch")? {
            let watching_vec = self.config.get_array("config.watch")?;
            let mut watching = Vec::new();
            for val in watching_vec {
                watching.push(val.into_str()?);
            }

            let (tx, rx) = mpsc::channel();
            let mut watcher = notify::watcher(tx, Duration::from_secs(10))?;

            todo!()
        } else {
            let path = self.config.get_str("path")?;
            self.run_on_path(path, None)
        }
    }

    fn run_on_path(
        &self,
        path: impl AsRef<Path>,
        library: Option<&str>,
    ) -> Result<(), Box<dyn StdError>> {
        let format = if let Some(library) = library {
            self.config
                .get_str(&format!("libraries.{}.format", library))?
        } else {
            self.config.get_str("format")?
        };

        read_dir(path)?.par_bridge().for_each(|entry| {
            let entry = entry.expect("Cannot get entry!");
            let file_type = entry.file_type().expect("Cannot get file type!");

            if !file_type.is_file() {
                return;
            }

            if let Some(err) = self.organize_file(&entry, &format).err() {
                let err = err.downcast::<Error>().unwrap();
                eprintln! {
                    "Failed on: {} ({})",
                    entry.path().to_str().unwrap_or("{unknown path}"),
                    err
                };
            }
        });

        Ok(())
    }

    fn organize_file(&self, entry: &DirEntry, format: &str) -> Result<(), Box<dyn StdError>> {
        let metadata = metadata::Metadata::from_path(entry.path())?;
        let path = metadata.build_path(format).ok_or(Error::MissingValues)?;
        let cwd = entry.path().parent().ok_or(Error::BadParent)?.to_owned();

        if self.config.get_bool("dryrun")? {
            println!(
                "Dry run on: {}\n  Item created: {}",
                entry.path().to_str().unwrap_or("Invalid unicode path!"),
                path.to_str().unwrap_or("Invalid unicode path")
            );
        } else {
            println!(
                "Working on: {}\n  Item created: {}",
                entry.path().to_str().unwrap_or("Invalid unicode path!"),
                path.to_str().unwrap_or("Invalid unicode path")
            );

            maybe_create_dir(cwd.join(&path.parent().ok_or(Error::BadParent)?))?;
            rename(entry.path(), cwd.join(&path))?;
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

fn expand_paths(config: &mut Config) -> Result<(), Box<dyn StdError>> {
    let libraries = config.get_table("libraries")?;

    for (library, table) in libraries {
        let table = table.into_table()?;
        let folders = table
            .get("folders")
            .ok_or_else(|| Error::MissingConfigProperty(format!("{}.folders", library)))?;
    }

    todo!()
}
