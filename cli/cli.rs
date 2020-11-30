use std::env;
use std::path::PathBuf;
use std::str::FromStr;

use clap::Clap;
use muso::config::Config;
use muso::format::ParsedFormat;
use muso::sorting::Options;
use nom::combinator::ParserIterator;

const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const ABOUT: &str = env!("CARGO_PKG_DESCRIPTION");

#[derive(Debug, Clap)]
#[clap(name = "muso")]
#[clap(about = ABOUT)]
#[clap(author = AUTHORS)]
pub struct CliArgs {
    /// Path to custom config file.
    #[clap(short, long)]
    pub config: Option<PathBuf>,

    #[clap(subcommand)]
    pub cmd: SubCommand,
}

#[derive(Debug, Clap)]
pub enum SubCommand {
    /// Copy service file to systemd user config dir.
    #[clap(name = "copy-service")]
    CopyService,

    /// Watch libraries and sort added files.
    Watch,

    /// Sort a music directory.
    Sort {
        /// Path to music directory.
        #[clap(parse(try_from_str = parse_path))]
        path: PathBuf,

        /// Custom format string
        #[clap(short, long)]
        format: Option<String>,

        /// Don't sort anything. Simulated run.
        #[clap(short, long)]
        dryrun: bool,

        /// Sort files recursively.
        #[clap(short, long)]
        recursive: bool,

        /// Remove empty directories found while and after sorting.
        #[clap(short, long)]
        remove_empty: bool,

        /// Mantain file names compatible with FAT32.
        #[clap(short, long)]
        exfat_compat: bool,
    },

    /// Goodies related to sync mode.
    #[cfg(feature = "sync")]
    Sync,
}

fn parse_path(path: &str) -> Result<PathBuf, &'static str> {
    todo!()
}

/*
impl SubCommand {
    pub fn build_sort_options(&self, config: &Config) -> Option<Options<ParsedFormat>> {
        match self {
            Self::Sort { path, format, .. } => {
                let format = format
                    .clone()
                    .map_or(config.search_format(&path).cloned(), |s| {
                        ParsedFormat::from_str(&s).ok()
                    })
                    .unwrap_or_else(|| {
                        ParsedFormat::from_str("{artist}/{album}/{track} - {title}.{ext}").unwrap()
                    });

                Some(Options {
                    format,
                    dryrun: (),
                    recursive: (),
                    exfat_compat: (),
                    remove_empty: (),
                })
            }
            _ => None,
        }
    }
}
*/
