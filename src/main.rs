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
        (@arg path: !required "The path where muso will work")
        (@arg watch: -w --watch "Run in watcher mode")
        (@arg format: -f --format +takes_value "Specifies format to save")
        (@arg dryrun: -d --dryrun "Don\'t organize files but show created paths")
        (@arg config: -C --config +takes_value "Custom config file location")
        (@arg recursive: -r --recursive "Perform sorting recursively")
        (@arg clean: -c --clean "Clean empty folders at the end")
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
