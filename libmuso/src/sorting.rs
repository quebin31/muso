use std::borrow::Cow;
use std::path::Path;
use std::{fs, path::PathBuf};

use crate::format::ParsedFormat;
use crate::metadata::Metadata;
use crate::utils;
use crate::{Error, Result};

#[derive(Debug, Clone)]
pub struct Options<'a> {
    pub format: Cow<'a, ParsedFormat>,
    pub dryrun: bool,
    pub recursive: bool,
    pub exfat_compat: bool,
    pub remove_empty: bool,
}

#[derive(Debug, Clone)]
pub struct SortReport {
    pub success: usize,
    pub total: usize,
    pub new_paths: Vec<PathBuf>,
}

pub fn sort_folder<R, D>(root: R, dir: D, options: &Options) -> Result<SortReport>
where
    R: AsRef<Path>,
    D: AsRef<Path>,
{
    let mut report = SortReport {
        success: 0,
        total: 0,
        new_paths: Vec::new(),
    };

    let dir = dir.as_ref().to_path_buf();
    let mut stack = vec![dir];

    while let Some(path) = stack.pop() {
        let metadata = match fs::metadata(&path) {
            Ok(metadata) => metadata,
            Err(e) => {
                log::error!(
                    "Couldn't read metadata from: \"{}\" ({})",
                    path.display(),
                    e
                );
                continue;
            }
        };

        if metadata.is_file() {
            match sort_file(&root, path, options) {
                Ok(new_path) => {
                    report.success += 1;
                    report.total += 1;
                    report.new_paths.push(new_path);
                }

                Err(e) => {
                    log::error!("{}", e);
                    report.total += 1;
                }
            }

            continue;
        }

        match fs::read_dir(&path) {
            Ok(entries) => {
                let mut len = 0;

                for entry in entries {
                    match entry {
                        Ok(entry) => {
                            len += 1;
                            stack.push(entry.path());
                        }

                        Err(e) => {
                            log::error!("{}", e);
                        }
                    }
                }

                if options.remove_empty && len == 0 {
                    log::info!("Removing empty folder: \"{}\"", path.display());
                    if let Err(e) = fs::remove_dir(path) {
                        log::error!("Couldn't remove dir ({})", e);
                    }
                }
            }

            Err(e) => {
                log::error!("{}", e);
            }
        }
    }

    Ok(report)
}

pub fn sort_file<R, F>(root: R, file: F, options: &Options) -> Result<PathBuf>
where
    R: AsRef<Path>,
    F: AsRef<Path>,
{
    if options.dryrun {
        log::info!("Working on (dryrun): \"{}\"", file.as_ref().display());
    } else {
        log::info!("Working on: \"{}\"", file.as_ref().display());
    }

    let metadata = Metadata::from_path(&file)?;
    let new_path = options.format.build_path(&metadata, options.exfat_compat)?;

    if !options.dryrun {
        let new_path = root.as_ref().join(&new_path);
        let new_path_parent = new_path.parent().ok_or(Error::InvalidParent {
            child: new_path.to_string_lossy().into(),
        })?;

        utils::maybe_create_dir(new_path_parent)?;
        fs::rename(&file, &new_path)?;
    }

    log::info!("Item created: \"{}\"", new_path.display());

    Ok(new_path)
}
