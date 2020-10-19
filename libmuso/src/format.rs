// Copyright (C) 2020 Kevin Dc
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

mod parser;

use std::result::Result as StdResult;
use std::{path::PathBuf, str::FromStr};

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use self::parser::parse_format_string;
use self::parser::{BasicComponent, FsComponent};
use self::parser::{Placeholder, Tag};

use crate::metadata::Metadata;
use crate::{Error, Result};

#[derive(Debug, Clone)]
pub struct ParsedFormat {
    fs_components: Vec<FsComponent>,
    orig_string: String,
}

impl FromStr for ParsedFormat {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let basic_components = parse_format_string(s)?;

        let mut fs_component = Vec::new();
        let mut fs_components = Vec::new();

        for component in basic_components {
            match component {
                BasicComponent::String(s) => {
                    let mut splitted: Vec<_> = s.split('/').collect();

                    for part in splitted.drain(0..(splitted.len() - 1)) {
                        if !part.is_empty() {
                            fs_component.push(BasicComponent::String(part.into()));
                        }

                        fs_components.push(FsComponent::Dir(fs_component.clone()));
                        fs_component.clear();
                    }

                    if !splitted[0].is_empty() {
                        fs_component.push(BasicComponent::String(splitted[0].into()));
                    }
                }

                placeholder => fs_component.push(placeholder),
            }
        }

        if !fs_component.is_empty() {
            fs_components.push(FsComponent::File(fs_component));
        }

        Ok(Self {
            fs_components,
            orig_string: s.to_string(),
        })
    }
}

struct ParsedFormatVisitor;

impl<'d> Visitor<'d> for ParsedFormatVisitor {
    type Value = ParsedFormat;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Expecting an string")
    }

    fn visit_str<E>(self, v: &str) -> StdResult<Self::Value, E>
    where
        E: de::Error,
    {
        ParsedFormat::from_str(v).map_err(de::Error::custom)
    }
}

impl<'d> Deserialize<'d> for ParsedFormat {
    fn deserialize<D>(deserializer: D) -> StdResult<Self, D::Error>
    where
        D: Deserializer<'d>,
    {
        deserializer.deserialize_str(ParsedFormatVisitor)
    }
}

impl Serialize for ParsedFormat {
    fn serialize<S>(&self, serializer: S) -> StdResult<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.orig_string)
    }
}

impl ParsedFormat {
    pub fn build_path(&self, metadata: &Metadata, exfat_compat: bool) -> Result<PathBuf> {
        let mut path = String::with_capacity(128);

        for fs_component in &self.fs_components {
            match fs_component {
                FsComponent::Dir(dir) => {
                    for component in dir {
                        match component {
                            BasicComponent::String(s) => {
                                path.push_str(s);
                            }

                            BasicComponent::Placeholder(p) => {
                                let s = Self::get_from_metadata(metadata, *p)?
                                    .ok_or_else(|| Error::OptionalInDir)?;

                                path.push_str(&Self::replace(s, exfat_compat));
                            }
                        }
                    }

                    path.push('/');
                }

                FsComponent::File(file) => {
                    let mut required_founds = 0;
                    for component in file {
                        match component {
                            BasicComponent::String(s) => {
                                path.push_str(s);
                            }

                            BasicComponent::Placeholder(p) => {
                                if !p.is_optional() && !p.is_tag(Tag::Ext) {
                                    required_founds += 1;
                                }

                                if let Some(s) = Self::get_from_metadata(metadata, *p)? {
                                    path.push_str(&Self::replace(s, exfat_compat));
                                }
                            }
                        }
                    }

                    if required_founds < 1 {
                        return Err(Error::RequiredInFile);
                    }
                }
            }
        }

        Ok(PathBuf::from(path))
    }

    fn replace(string: String, exfat_compat: bool) -> String {
        if exfat_compat {
            string
                .replace('/', "_")
                .replace('"', "_")
                .replace('*', "_")
                .replace(':', "_")
                .replace('<', "_")
                .replace('>', "_")
                .replace('\\', "_")
                .replace('?', "_")
                .replace('|', "_")
        } else {
            string.replace('/', "_")
        }
    }

    fn add_leading_zeros(string: String, leading: u8) -> String {
        if (leading as usize) > string.len() {
            let mut res: String = vec!['0'; leading as usize - string.len()].iter().collect();
            res.push_str(&string);
            res
        } else {
            string
        }
    }

    fn get_from_metadata(metadata: &Metadata, pholder: Placeholder) -> Result<Option<String>> {
        let is_optional = pholder.is_optional();
        let tag = pholder.into_tag();

        match tag {
            Tag::Artist => match metadata.get_artist() {
                Ok(artist) => Ok(Some(artist)),
                Err(_) if is_optional => Ok(None),
                Err(e) => Err(e),
            },

            Tag::Album => match metadata.get_album() {
                Ok(album) => Ok(Some(album)),
                Err(_) if is_optional => Ok(None),
                Err(e) => Err(e),
            },

            Tag::Disc { leading } => match metadata.get_disc() {
                Ok(disc) => Ok(Some(Self::add_leading_zeros(disc, leading))),
                Err(_) if is_optional => Ok(None),
                Err(e) => Err(e),
            },

            Tag::Track { leading } => match metadata.get_track() {
                Ok(track) => Ok(Some(Self::add_leading_zeros(track, leading))),
                Err(_) if is_optional => Ok(None),
                Err(e) => Err(e),
            },

            Tag::Title => match metadata.get_title() {
                Ok(title) => Ok(Some(title)),
                Err(_) if is_optional => Ok(None),
                Err(e) => Err(e),
            },

            Tag::Ext => Ok(Some(metadata.get_ext())),
        }
    }
}
