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
use std::path::Path;
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
        } => {
            let path = path.unwrap_or(env::current_dir()?);
            let format = format
                .map_or(config.search_format(&path).cloned(), |s| {
                    ParsedFormat::from_str(&s).ok()
                })
                .unwrap_or_else(|| {
                    ParsedFormat::from_str("{artist}/{album}/{track} - {title}.{ext}").unwrap()
                });

            let options = Options {
                format,
                dryrun,
                recursive,
                exfat_compat,
                remove_empty,
            };

            if path.is_dir() {
                match sort_folder(&path, &path, &options) {
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
                    path: path.display().to_string(),
                };

                return Err(err.into());
            }
        }

        #[cfg(feature = "sync")]
        SubCommand::Sync => {}
    }

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
