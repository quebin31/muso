use std::fmt;
use std::fs::File;
use std::hash::Hash;
use std::io::Read;
use std::path::Path;
use std::result::Result as StdResult;

use serde::de::{self, Visitor};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{Error, Result};

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Sha256Sum(pub Vec<u8>);

impl Sha256Sum {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let mut file = File::open(path)?;
        let mut bytes = Vec::new();

        let _ = file.read_to_end(&mut bytes)?;

        let mut hasher = Sha256::new();
        hasher.update(bytes);

        Ok(Self::from_hasher(hasher))
    }

    pub fn from_hasher(hasher: Sha256) -> Self {
        let sum = hasher.finalize();
        Sha256Sum(sum[..].to_vec())
    }
}

impl Serialize for Sha256Sum {
    fn serialize<S>(&self, serializer: S) -> StdResult<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

pub struct Sha256SumVisitor;

impl<'d> Visitor<'d> for Sha256SumVisitor {
    type Value = Sha256Sum;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Expecting 32 bytes sequence")
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> StdResult<Self::Value, E>
    where
        E: de::Error,
    {
        if v.len() == 32 {
            Ok(Sha256Sum(v))
        } else {
            Err(de::Error::custom(Error::InvalidSha256))
        }
    }
}

impl<'d> Deserialize<'d> for Sha256Sum {
    fn deserialize<D>(deserializer: D) -> StdResult<Self, D::Error>
    where
        D: serde::Deserializer<'d>,
    {
        deserializer.deserialize_byte_buf(Sha256SumVisitor)
    }
}

pub fn sha256_for_bytes(bytes: impl AsRef<[u8]>) -> Sha256Sum {
    let mut hasher = Sha256::new();
    hasher.update(bytes.as_ref());

    let result = hasher.finalize();
    Sha256Sum(result[..].to_vec())
}
