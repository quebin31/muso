use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ssh2::Session;
use try_block::try_block;
use walkdir::WalkDir;

use crate::sync::sha256::Sha256Sum;
use crate::{Error, Result};

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub enum HostType {
    Primary,
    Replica,
}

#[derive(Debug, Clone)]
pub enum Diff<T> {
    Added(T),
    Removed(T),
}

pub type Differences<'a> = Vec<Diff<(&'a Sha256Sum, &'a Path)>>;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct State {
    devtype: HostType,
    paths: HashMap<Sha256Sum, PathBuf>,
    modification_date: DateTime<Utc>,
}

impl State {
    // 500 Kb buffer size
    pub const MAX_NEEDED_BYTES: usize = 500 * 1024;

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

    pub fn init_on_primary(root: impl AsRef<Path>) -> Result<Self> {
        let mut paths = HashMap::new();
        let walkdir = WalkDir::new(root).into_iter().filter_map(|e| e.ok());

        for entry in walkdir {
            let path = entry.path();

            let sha256sum: Result<Sha256Sum> = try_block! {
                let mut file = File::open(&path)?;
                let mut bytes = [0u8; Self::MAX_NEEDED_BYTES];
                let len = file.read(&mut bytes)?;

                Ok(Sha256Sum::from_bytes(&bytes[..len]))
            };

            if let Ok(sha256sum) = sha256sum {
                paths.insert(sha256sum, path.to_path_buf());
            }
        }

        Ok(State {
            devtype: HostType::Primary,
            paths,
            modification_date: Utc::now(),
        })
    }

    pub fn init_on_replica<A>(root: impl AsRef<Path>, addr: A) -> Result<Self>
    where
        A: ToSocketAddrs,
    {
        let tcp_stream = TcpStream::connect(addr)?;
        let mut session = Session::new()?;
        session.set_tcp_stream(tcp_stream);
        session.handshake()?;

        session.userauth_password("musosync", "musosyncpass")?;
        if !session.authenticated() {
            return Err(Error::SshAuthFail);
        }

        let sftp = session.sftp()?;
        todo!("walkdir sftp")
    }

    pub fn differences<'a>(&'a self, other: &'a Self) -> Result<Differences> {
        if self.devtype == other.devtype {
            return Err(Error::InvalidStateDiff);
        }

        let mut diffs = Vec::new();

        let primary;
        let replica;
        if let HostType::Primary = self.devtype {
            primary = &self;
            replica = &other;
        } else {
            primary = &other;
            replica = &self;
        }

        for (primary_key, primary_value) in &primary.paths {
            if !replica.paths.contains_key(primary_key) {
                diffs.push(Diff::Added((primary_key, primary_value.as_path())));
            }
        }

        for (replica_key, replica_value) in &replica.paths {
            if !primary.paths.contains_key(replica_key) {
                diffs.push(Diff::Removed((replica_key, replica_value.as_path())));
            }
        }

        Ok(diffs)
    }
}
