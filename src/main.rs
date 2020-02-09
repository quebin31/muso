mod error;
mod metadata;
mod muso;

use std::error::Error as StdError;

use clap::clap_app;

use crate::muso::Muso;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const ABOUT: &str = env!("CARGO_PKG_DESCRIPTION");

fn main() -> Result<(), Box<dyn StdError>> {
    let matches = clap_app! { muso =>
        (version: VERSION)
        (author: AUTHORS)
        (about: ABOUT)
        (@arg path: !required "The path where muso will work")
        (@arg watch: -w --watch "Run in watcher mode")
        (@arg format: -f --format +takes_value "Specifies format to save")
        (@arg dryrun: -d --dryrun "Don\'t organize files but show created paths")
        (@arg config: -c --config +takes_value "Custom config file location")
    }
    .get_matches();

    Muso::new(&matches)?.run()?;
    println!("Done!");
    Ok(())
}
