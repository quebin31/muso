pub mod sha256;

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use libmtp_rs::object::AsObjectId;
use libmtp_rs::storage::Storage;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use self::sha256::Sha256Sum;
use crate::Result;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum DevType {
    Primary,
    Replica,
}

#[derive(Debug, Clone)]
pub enum Diff<T> {
    Added(T),
    Removed(T),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum SyncPath {
    PathBuf(PathBuf),
    MtpPath(Vec<u32>),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SyncInfo {
    dev_type: DevType,
    paths: HashMap<Sha256Sum, SyncPath>,
    modification_date: DateTime<Utc>,
}

impl SyncInfo {
    pub fn init_on_primary(root: impl AsRef<Path>) -> Result<Self> {
        let mut paths = HashMap::new();
        let walkdir = WalkDir::new(root).into_iter().filter_map(|e| e.ok());

        for entry in walkdir {
            let path = entry.path();

            if let Ok(sha256sum) = Sha256Sum::from_file(path) {
                paths.insert(sha256sum, SyncPath::PathBuf(path.to_path_buf()));
            }
        }

        Ok(SyncInfo {
            dev_type: DevType::Primary,
            paths,
            modification_date: Utc::now(),
        })
    }

    pub fn init_on_replica(storage: Storage, root: impl AsObjectId) -> Result<Self> {
        todo!()
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let mut file = File::open(path)?;
        let mut bytes = Vec::new();

        let _ = file.read_to_end(&mut bytes)?;
        Self::from_bytes(bytes)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let bytes = self.to_bytes()?;

        let mut file = File::create(path)?;
        let _ = file.write(&bytes)?;
        Ok(())
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        Ok(bincode::serialize(&self)?)
    }

    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Result<Self> {
        Ok(bincode::deserialize(bytes.as_ref())?)
    }

    pub fn differences<'a>(
        &'a self,
        replica: &'a Self,
    ) -> Vec<Diff<(&'a Sha256Sum, &'a SyncPath)>> {
        let mut diffs = Vec::new();

        for (primary_key, primary_value) in &self.paths {
            if !replica.paths.contains_key(primary_key) {
                diffs.push(Diff::Added((primary_key, primary_value)));
            }
        }

        for (replica_key, replica_value) in &self.paths {
            if !self.paths.contains_key(replica_key) {
                diffs.push(Diff::Removed((replica_key, replica_value)));
            }
        }

        diffs
    }
}
