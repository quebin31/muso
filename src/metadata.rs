// Copyright (C) 2020 kevin
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

use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use common_macros::hash_set;
use failure::Error;
use infer;
use lazy_static::lazy_static;

use crate::error::MusoError;

#[derive(Debug)]
pub struct Metadata {
    pub artist: Option<String>,
    pub album: Option<String>,
    pub disc: Option<u32>,
    pub track: Option<u32>,
    pub title: Option<String>,
    pub ext: String,
}

lazy_static! {
    static ref PLACEHOLDERS: HashSet<&'static str> = hash_set! {
        "{artist}", "{album}", "{disc}", "{track}", "{title}", "{ext}"
    };
}

macro_rules! get_placeholder {
    ($self:ident, $placeholder:ident, $exfat_compat:expr) => {{
        replace(
            &$self
                .$placeholder
                .as_ref()
                .ok_or_else(|| MusoError::MissingTagProperty(stringify!($placeholder).to_owned()))?
                .to_string(),
            $exfat_compat,
        )
    }};
}

impl Metadata {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, Error> {
        let mut file = File::open(&path)?;
        let mut magic_bytes = [0; 4];
        file.read_exact(&mut magic_bytes)?;

        let infer = infer::Infer::new();
        let ftype = infer.get(&magic_bytes).ok_or(MusoError::NotSupported)?;
        match ftype.mime.as_str() {
            "audio/x-flac" => Metadata::from_vorbis(&path),
            "audio/mpeg" => Metadata::from_id3(&path),
            _ => Err(MusoError::NotSupported.into()),
        }
    }

    fn from_id3(path: impl AsRef<Path>) -> Result<Self, Error> {
        let tag = id3::Tag::read_from_path(path)?;

        let artist = if let Some(artist) = tag.album_artist() {
            Some(artist.to_owned())
        } else {
            tag.artist().map(|s| s.to_owned())
        };

        let album = tag.album().map(|s| s.to_owned());
        let disc = tag.disc();
        let track = tag.track();
        let title = tag.title().map(|s| s.to_owned());

        Ok(Metadata {
            artist,
            album,
            disc,
            track,
            title,
            ext: "mp3".to_owned(),
        })
    }

    fn from_vorbis(path: impl AsRef<Path>) -> Result<Self, Error> {
        let tag = metaflac::Tag::read_from_path(path)?;
        let comments = &tag
            .vorbis_comments()
            .ok_or(MusoError::EmptyComments)?
            .comments;

        let artist = if let Some(artist) = comments.get("ALBUMARTIST").and_then(|a| a.get(0)) {
            Some(artist.to_owned())
        } else {
            comments
                .get("ARTIST")
                .map(|a| a.get(0).map(|s| s.to_owned()))
                .flatten()
        };

        let album = comments
            .get("ALBUM")
            .map(|a| a.get(0).map(|s| s.to_owned()))
            .flatten();

        let disc = comments
            .get("DISCNUMBER")
            .map(|d| d.get(0).map(|s| s.parse::<u32>().ok()))
            .flatten()
            .flatten();

        let track = comments
            .get("TRACKNUMBER")
            .map(|t| t.get(0).map(|s| s.parse::<u32>().ok()))
            .flatten()
            .flatten();

        let title = comments
            .get("TITLE")
            .map(|t| t.get(0).map(|s| s.to_owned()))
            .flatten();

        Ok(Metadata {
            artist,
            album,
            disc,
            track,
            title,
            ext: "flac".to_owned(),
        })
    }

    pub fn build_path(&self, format: &str, exfat_compat: bool) -> Result<String, MusoError> {
        let mut path = format.to_owned();

        for placeholder in &*PLACEHOLDERS {
            if !path.contains(placeholder) {
                continue;
            }

            let value = match *placeholder {
                "{artist}" => get_placeholder!(self, artist, exfat_compat),
                "{album}" => get_placeholder!(self, album, exfat_compat),
                "{disc}" => get_placeholder!(self, disc, exfat_compat),
                "{track}" => get_placeholder!(self, track, exfat_compat),
                "{title}" => get_placeholder!(self, title, exfat_compat),
                "{ext}" => self.ext.clone(),
                forgotten => unreachable!("Unreacheable with {}", forgotten),
            };

            path = path.replace(*placeholder, &value);
        }

        Ok(path)
    }
}

fn replace(string: &str, exfat_compat: bool) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_flac_with_ok_format() {
        let metadata = Metadata::from_path("test_files/complete.flac").unwrap();

        assert_eq! {
            Ok("Album Artist/Album/1.1 - Title.flac".into()),
            metadata.build_path("{artist}/{album}/{disc}.{track} - {title}.{ext}", false)
        };
    }

    #[test]
    fn partial_flac_with_ok_format() {
        let metadata = Metadata::from_path("test_files/partial.flac").unwrap();

        assert_eq! {
            Ok("Artist/1.1 - Title.flac".into()),
            metadata.build_path("{artist}/{disc}.{track} - {title}.{ext}", false)
        };
    }

    #[test]
    fn partial_flac_with_bad_format() {
        let metadata = Metadata::from_path("test_files/partial.flac").unwrap();

        assert_eq! {
            Err(MusoError::MissingTagProperty("album".to_owned())),
            metadata.build_path("{artist}/{album}", false)
        };
    }

    #[test]
    fn complete_mp3_with_ok_format() {
        let metadata = Metadata::from_path("test_files/complete.mp3").unwrap();

        assert_eq! {
            Ok("Album Artist/Album/1.1 - Title.mp3".into()),
            metadata.build_path("{artist}/{album}/{disc}.{track} - {title}.{ext}", false)
        };
    }

    #[test]
    fn partial_mp3_with_ok_format() {
        let metadata = Metadata::from_path("test_files/partial.mp3").unwrap();

        assert_eq! {
            Ok("Artist/1.1 - Title.mp3".into()),
            metadata.build_path("{artist}/{disc}.{track} - {title}.{ext}", false)
        };
    }

    #[test]
    fn partial_mp3_with_bad_format() {
        let metadata = Metadata::from_path("test_files/partial.mp3").unwrap();

        assert_eq! {
            Err(MusoError::MissingTagProperty("album".to_owned())),
            metadata.build_path("{artist}/{album}", false)
        };
    }
}
