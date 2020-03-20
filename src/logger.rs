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

use ansi_term::Color::{Cyan, Red, Yellow};
use log::{set_logger, set_max_level, Level, LevelFilter, Log, Metadata, Record, SetLoggerError};

pub struct MusoLogger;

static MUSO_LOGGER: MusoLogger = MusoLogger {};

pub fn init_logger() -> Result<(), SetLoggerError> {
    set_logger(&MUSO_LOGGER).map(|_| set_max_level(LevelFilter::Info))
}

impl Log for MusoLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        match record.level() {
            Level::Info => println!("{} {}", Cyan.bold().paint("[i]"), record.args()),
            Level::Warn => eprintln!("{} {}", Yellow.bold().paint("[w]"), record.args()),
            Level::Error => eprintln!("{} {}", Red.bold().paint("[e]"), record.args()),
            _ => {}
        }
    }

    fn flush(&self) {}
}
