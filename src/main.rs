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
mod metadata;
mod muso;

use std::process;

use clap::clap_app;
use log::error;

use crate::logger::init_logger;
use crate::muso::Muso;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const ABOUT: &str = env!("CARGO_PKG_DESCRIPTION");

fn main() {
    init_logger().unwrap();

    let matches = clap_app! { muso =>
        (version: VERSION)
        (author: AUTHORS)
        (about: ABOUT)
        (@arg path: !required "Working path to sort")
        (@arg format: -f --format +takes_value "Custom format string")
        (@arg config: -C --config +takes_value "Custom config file location")
        (@arg watch: -w --watch "Watch libraries present in config")
        (@arg dryrun: -d --dryrun "Don\'t create neither move anything")
        (@arg recursive: -r --recursive "Search for files recursively")
        (@arg exfatcompat: --("exfat-compat") "Maintain names compatible with FAT32")
        (@arg copyservice: --("copy-service") conflicts_with[format config
            watch dryrun recursive exfatcompat path]
            "Copy service file to systemd user config dir, nothing else")
    }
    .get_matches();

    process::exit(match Muso::run(&matches) {
        Ok(_) => 0,
        Err(e) => {
            error!("{}", e);
            1
        }
    })
}
