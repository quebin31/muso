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
            Level::Info => println!("{} {}", Cyan.bold().paint("[info]"), record.args()),
            Level::Warn => eprintln!("{} {}", Yellow.bold().paint("[warn]"), record.args()),
            Level::Error => eprintln!("{} {}", Red.bold().paint("[error]"), record.args()),
            _ => {}
        }
    }

    fn flush(&self) {}
}
