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

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::error::{AnyResult, MusoError, MusoResult};

#[derive(Debug)]
pub struct Metadata {
    pub artist: Option<String>,
    pub album: Option<String>,
    pub disc: Option<u32>,
    pub track: Option<u32>,
    pub title: Option<String>,
    pub ext: String,
}

macro_rules! impl_tag_getter {
    ($self:ident, $tag:ident) => {
        $self
            .$tag
            .as_ref()
            .ok_or_else(|| MusoError::MissingTag {
                tag: stringify!($tag).into(),
            })
            .map(|s| s.to_string())
    };
}

impl Metadata {
    pub fn from_path(path: impl AsRef<Path>) -> AnyResult<Self> {
        let mut file = File::open(&path)?;
        // NOTE(erichdongubler): This could be smaller if media types with larger magic bytes
        // length requirements for `infer` get removed, so let's keep a table below of length
        // required for each.
        let mut magic_bytes = [0; 4];
        file.read_exact(&mut magic_bytes)
            .map_err(|_| MusoError::NotSupported)?;

        let infer = infer::Infer::new();
        let ftype = infer.get(&magic_bytes).ok_or(MusoError::NotSupported)?;
        match ftype.mime.as_str() {
            // Minimum: 4 bytes
            "audio/x-flac" => Metadata::from_flac_vorbis(&path),
            // Minimum: 4 bytes
            "audio/mpeg" => Metadata::from_id3(&path),
            // Minimum: 4 bytes
            "audio/ogg" => Metadata::from_ogg_vorbis(&path),
            _ => Err(MusoError::NotSupported.into()),
        }
    }

    fn from_id3(path: impl AsRef<Path>) -> AnyResult<Self> {
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

    fn from_flac_vorbis(path: impl AsRef<Path>) -> AnyResult<Self> {
        let tag = metaflac::Tag::read_from_path(path)?;
        let comments = tag
            .vorbis_comments()
            .ok_or(MusoError::EmptyComments)?
            .comments
            .to_owned();

        Self::from_vorbis_comments(comments, "flac")
    }

    fn from_ogg_vorbis(path: impl AsRef<Path>) -> AnyResult<Self> {
        let file = File::open(path)?;
        let mut reader = ogg::reading::PacketReader::new(file);
        let ((_, comments, _), _) = lewton::inside_ogg::read_headers(&mut reader)?;
        let comments = Self::ogg_comment_map(comments.comment_list);

        Self::from_vorbis_comments(comments, "ogg")
    }

    fn from_vorbis_comments(comments: HashMap<String, Vec<String>>, ext: &str) -> AnyResult<Self> {
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
            ext: ext.to_owned(),
        })
    }

    fn ogg_comment_map(list: Vec<(String, String)>) -> HashMap<String, Vec<String>> {
        let mut map = HashMap::new();

        for (key, value) in list {
            let entry = map.entry(key).or_insert_with(Vec::new);
            entry.push(value);
        }

        map
    }

    pub fn get_artist(&self) -> MusoResult<String> {
        impl_tag_getter!(self, artist)
    }

    pub fn get_album(&self) -> MusoResult<String> {
        impl_tag_getter!(self, album)
    }

    pub fn get_disc(&self) -> MusoResult<String> {
        impl_tag_getter!(self, disc)
    }

    pub fn get_track(&self) -> MusoResult<String> {
        impl_tag_getter!(self, track)
    }

    pub fn get_title(&self) -> MusoResult<String> {
        impl_tag_getter!(self, title)
    }

    pub fn get_ext(&self) -> String {
        self.ext.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_flac() {
        let metadata = Metadata::from_path("test_files/complete.flac").unwrap();

        assert_eq!(Ok("Album Artist".into()), metadata.get_artist());
        assert_eq!(Ok("Album".into()), metadata.get_album());
        assert_eq!(Ok("1".into()), metadata.get_disc());
        assert_eq!(Ok("1".into()), metadata.get_track());
        assert_eq!(Ok("Title".into()), metadata.get_title());
        assert_eq!("flac".to_owned(), metadata.get_ext());
    }

    #[test]
    fn partial_flac() {
        let metadata = Metadata::from_path("test_files/partial.flac").unwrap();

        assert_eq!(Ok("Artist".into()), metadata.get_artist());
        assert_eq!(
            Err(MusoError::MissingTag {
                tag: "album".into()
            }),
            metadata.get_album()
        );
        assert_eq!(Ok("1".into()), metadata.get_disc());
        assert_eq!(Ok("1".into()), metadata.get_track());
        assert_eq!(Ok("Title".into()), metadata.get_title());
        assert_eq!("flac".to_owned(), metadata.get_ext());
    }

    #[test]
    fn complete_mp3() {
        let metadata = Metadata::from_path("test_files/complete.mp3").unwrap();

        assert_eq!(Ok("Album Artist".into()), metadata.get_artist());
        assert_eq!(Ok("Album".into()), metadata.get_album());
        assert_eq!(Ok("1".into()), metadata.get_disc());
        assert_eq!(Ok("1".into()), metadata.get_track());
        assert_eq!(Ok("Title".into()), metadata.get_title());
        assert_eq!("mp3".to_owned(), metadata.get_ext());
    }

    #[test]
    fn partial_mp3() {
        let metadata = Metadata::from_path("test_files/partial.mp3").unwrap();

        assert_eq!(Ok("Artist".into()), metadata.get_artist());
        assert_eq!(
            Err(MusoError::MissingTag {
                tag: "album".into()
            }),
            metadata.get_album()
        );
        assert_eq!(Ok("1".into()), metadata.get_disc());
        assert_eq!(Ok("1".into()), metadata.get_track());
        assert_eq!(Ok("Title".into()), metadata.get_title());
        assert_eq!("mp3".to_owned(), metadata.get_ext());
    }

    #[test]
    fn complete_ogg() {
        let metadata = Metadata::from_path("test_files/complete.ogg").unwrap();

        assert_eq!(Ok("Album Artist".into()), metadata.get_artist());
        assert_eq!(Ok("Album".into()), metadata.get_album());
        assert_eq!(Ok("1".into()), metadata.get_disc());
        assert_eq!(Ok("1".into()), metadata.get_track());
        assert_eq!(Ok("Title".into()), metadata.get_title());
        assert_eq!("ogg".to_owned(), metadata.get_ext());
    }

    #[test]
    fn partial_ogg() {
        let metadata = Metadata::from_path("test_files/partial.ogg").unwrap();

        assert_eq!(Ok("Artist".into()), metadata.get_artist());
        assert_eq!(
            Err(MusoError::MissingTag {
                tag: "album".into()
            }),
            metadata.get_album()
        );
        assert_eq!(Ok("1".into()), metadata.get_disc());
        assert_eq!(Ok("1".into()), metadata.get_track());
        assert_eq!(Ok("Title".into()), metadata.get_title());
        assert_eq!("ogg".to_owned(), metadata.get_ext());
    }
}
