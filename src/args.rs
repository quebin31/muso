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

use std::env;
use std::path::PathBuf;

use clap::ArgMatches;

use crate::config::Config;
use crate::error::Result;

#[derive(Debug, Clone)]
pub struct Args {
    pub working_path: PathBuf,
    pub format: String,
    pub watch_mode: bool,
    pub dryrun: bool,
    pub recursive: bool,
    pub exfat_compat: bool,
}

impl Args {
    pub fn from_matches(matches: ArgMatches, config: &Config) -> Result<Self> {
        let working_path: PathBuf = matches
            .value_of("path")
            .map_or(env::current_dir()?.to_string_lossy().into(), |path| {
                path.to_string()
            })
            .into();

        let format: String = matches
            .value_of("format")
            .map_or(config.search_format_for(&working_path), |f| Some(f))
            .unwrap_or_else(|| "{artist}/{album}/{track} - {title}.{ext}")
            .into();

        let watch_mode = matches.is_present("watch");
        let dryrun = matches.is_present("dryrun");
        let recursive = matches.is_present("recursive");
        let exfat_compat = matches.is_present("exfatcompat");

        Ok(Self {
            working_path,
            format,
            watch_mode,
            dryrun,
            recursive,
            exfat_compat,
        })
    }
}
