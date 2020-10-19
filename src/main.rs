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

mod error;
mod logger;

use std::borrow::Cow;
use std::env;
use std::path::{Path, PathBuf};
use std::process;
use std::str::FromStr;

use clap::{App, Arg, ArgMatches};
use human_panic::setup_panic;
use libmuso::config::Config;
use libmuso::format::ParsedFormat;
use libmuso::sorting::{sort_folder, Options};
use libmuso::utils;
use libmuso::watcher::Watcher;

use crate::error::Error;
use crate::logger::init_logger;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const ABOUT: &str = env!("CARGO_PKG_DESCRIPTION");

pub type AnyResult<T> = std::result::Result<T, anyhow::Error>;

fn load_config(path: impl AsRef<Path>) -> AnyResult<Config> {
    let path = path.as_ref();
    let default_path = utils::default_config_path();

    if path == default_path && !path.exists() {
        cfg_if::cfg_if! {
            if #[cfg(feature = "standalone")] {
                utils::generate_resource(utils::Resource::Config, Some(include_str!("../share/config.toml")))?;
            } else {
                utils::generate_resource(utils::Resource::Config, None)?;
            }
        };
    }

    Ok(Config::from_path(path)?)
}

fn build_options<'a>(matches: &ArgMatches, config: &Config) -> AnyResult<(PathBuf, Options<'a>)> {
    let working_path = matches
        .value_of_os("path")
        .map_or(env::current_dir()?, |path| Path::new(path).to_path_buf());

    let format = matches
        .value_of("format")
        .map_or(config.search_format(&working_path).cloned(), |f| {
            ParsedFormat::from_str(f).ok()
        })
        .unwrap_or_else(|| {
            ParsedFormat::from_str("{artist}/{album}/{track} - {title}.{ext}").unwrap()
        });

    let dryrun = matches.is_present("dryrun");
    let recursive = matches.is_present("recursive");
    let exfat_compat = matches.is_present("exfatcompat");
    let remove_empty = matches.is_present("rm-empty");

    let options = Options {
        format: Cow::Owned(format),
        dryrun,
        recursive,
        exfat_compat,
        remove_empty,
    };

    Ok((working_path, options))
}

fn run(app: App) -> AnyResult<()> {
    let matches = app.get_matches();

    match matches.subcommand() {
        ("copy-service", _) => {
            cfg_if::cfg_if! {
                if #[cfg(feature = "standalone")] {
                    utils::generate_resource(utils::Resource::Service, Some(include_str!("../share/muso.service")))?;
                } else {
                    utils::generate_resource(utils::Resource::Service, None)?;
                }
            };
        }

        ("watch", Some(matches)) => {
            let config = matches
                .value_of_os("config")
                .map(|p| Path::new(p).to_path_buf())
                .unwrap_or_else(utils::default_config_path);

            let config = load_config(config)?;
            let watcher = Watcher::new(config);

            watcher.watch()?
        }

        ("sort", Some(matches)) => {
            let config = matches
                .value_of_os("config")
                .map(|p| Path::new(p).to_path_buf())
                .unwrap_or_else(utils::default_config_path);

            let config = load_config(config)?;
            let (working_path, options) = build_options(&matches, &config)?;

            if working_path.is_dir() {
                match sort_folder(&working_path, &working_path, &options) {
                    Ok(report) => log::info!(
                        "Done: {} successful out of {} ({} failed)",
                        report.success,
                        report.total,
                        report.total - report.success
                    ),

                    Err(e) => return Err(e.into()),
                }
            } else {
                let err = Error::InvalidRoot {
                    path: working_path.display().to_string(),
                };

                return Err(err.into());
            }
        }

        _ => {}
    }

    Ok(())
}

fn main() {
    setup_panic!();
    init_logger().unwrap();

    let app = App::new("muso")
        .version(VERSION)
        .author(AUTHORS)
        .about(ABOUT)
        .subcommand(App::new("copy-service").about("Copy service file to systemd user config dir"))
        .subcommand(
            App::new("watch")
                .about("Watch and sort new files in the specified libraries in the config file")
                .arg(
                    Arg::with_name("config")
                        .short("c")
                        .long("config")
                        .value_name("path")
                        .help("Custom config file path"),
                ),
        )
        .subcommand(
            App::new("sort")
                .about("Sort a music directory")
                .arg(
                    Arg::with_name("path")
                        .required(false)
                        .value_name("path")
                        .help("Working path to sort"),
                )
                .arg(
                    Arg::with_name("format")
                        .short("f")
                        .long("format")
                        .value_name("string")
                        .help("Custom format string"),
                )
                .arg(
                    Arg::with_name("config")
                        .short("c")
                        .long("config")
                        .value_name("path")
                        .help("Custom config file path"),
                )
                .arg(
                    Arg::with_name("dryrun")
                        .short("d")
                        .long("dryrun")
                        .help("Don't create neither move anything"),
                )
                .arg(
                    Arg::with_name("recursive")
                        .short("r")
                        .long("recursive")
                        .help("Search for files recursively"),
                )
                .arg(
                    Arg::with_name("rm-empty")
                        .long("rm-empty")
                        .help("Remove any empty directory found while sorting"),
                )
                .arg(
                    Arg::with_name("exfatcompat")
                        .long("exfat-compat")
                        .help("Maintain names compatible with FAT32"),
                ),
        );

    process::exit(match run(app) {
        Err(e) => {
            log::error!("{}", e);
            1
        }
        Ok(_) => 0,
    })
}
