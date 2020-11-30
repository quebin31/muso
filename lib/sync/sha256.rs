use std::fmt;
use std::hash::Hash;
use std::result::Result as StdResult;

use serde::de::{self, Visitor};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::Error;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Sha256Sum {
    pub sum: Vec<u8>,
}

impl Sha256Sum {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(&bytes);

        let sum = hasher.finalize();
        let sum = sum[..].to_vec();
        Self { sum }
    }

    pub fn from_hasher(hasher: &mut Sha256) -> Self {
        let sum = hasher.finalize_reset();
        let sum = sum[..].to_vec();
        Self { sum }
    }
}

impl Serialize for Sha256Sum {
    fn serialize<S>(&self, serializer: S) -> StdResult<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.sum)
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
            Ok(Sha256Sum { sum: v })
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
