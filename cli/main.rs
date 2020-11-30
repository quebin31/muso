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

mod cli;
mod error;
mod logger;

use std::env;
use std::path::{Path, PathBuf};
use std::process;
use std::str::FromStr;

use clap::Clap;
use human_panic::setup_panic;
use muso::config::Config;
use muso::format::ParsedFormat;
use muso::sorting::{sort_folder, Options};
use muso::utils;
use muso::watcher::Watcher;

use crate::cli::{CliArgs, SubCommand};
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

/*
fn build_options(
    matches: &ArgMatches,
    config: &Config,
) -> AnyResult<(PathBuf, Options<ParsedFormat>)> {
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
        format,
        dryrun,
        recursive,
        exfat_compat,
        remove_empty,
    };

    Ok((working_path, options))
}
*/

fn run(opts: CliArgs) -> AnyResult<()> {
    let config = opts.config.unwrap_or_else(utils::default_config_path);
    let config = load_config(config)?;

    match opts.cmd {
        SubCommand::CopyService => {
            cfg_if::cfg_if! {
                if #[cfg(feature = "standalone")] {
                    utils::generate_resource(utils::Resource::Service, Some(include_str!("../share/muso.service")))?;
                } else {
                    utils::generate_resource(utils::Resource::Service, None)?;
                }
            };
        }

        SubCommand::Watch => Watcher::new(config).watch()?,

        SubCommand::Sort {
            path,
            format,
            dryrun,
            recursive,
            remove_empty,
            exfat_compat,
        } => {}

        #[cfg(feature = "sync")]
        SubCommand::Sync => {}
    }

    /*
    match matches.subcommand() {
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
    */

    Ok(())
}

fn main() {
    setup_panic!();
    init_logger().unwrap();

    let opts = CliArgs::parse();
    process::exit(match run(opts) {
        Err(e) => {
            log::error!("{}", e);
            1
        }

        Ok(_) => 0,
    })
}
