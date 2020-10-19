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

use crate::{Error, Result};

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
            .ok_or_else(|| Error::MissingTag {
                tag: stringify!($tag).into(),
            })
            .map(|s| s.to_string())
    };
}

impl Metadata {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let mut file = File::open(&path)?;
        // NOTE(erichdongubler): This could be smaller if media types with larger magic bytes
        // length requirements for `infer` get removed, so let's keep a table below of length
        // required for each.
        let mut magic_bytes = [0; 11];
        file.read_exact(&mut magic_bytes)
            .map_err(|_| Error::NotSupported)?;

        let infer = infer::Infer::new();
        let ftype = infer.get(&magic_bytes).ok_or(Error::NotSupported)?;
        match ftype.mime.as_str() {
            // Minimum: 4 bytes
            "audio/x-flac" => Metadata::from_flac_vorbis(&path),
            // Minimum: 4 bytes
            "audio/mpeg" => Metadata::from_id3(&path),
            // Minimum: 4 bytes
            "audio/ogg" => Metadata::from_ogg_vorbis(&path),
            // Minimum: 11 bytes (4 normally, 11 to include `m4p`)
            "audio/m4a" => Metadata::from_m4a(&path),
            // Unsupported file
            _ => Err(Error::NotSupported),
        }
    }

    fn from_id3(path: impl AsRef<Path>) -> Result<Self> {
        let tag = match id3::Tag::read_from_path(path) {
            Ok(tag) => tag,
            Err(err) => err.partial_tag.clone().ok_or_else(|| err)?,
        };

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

    fn from_flac_vorbis(path: impl AsRef<Path>) -> Result<Self> {
        let tag = metaflac::Tag::read_from_path(path)?;
        let comments = tag
            .vorbis_comments()
            .ok_or(Error::EmptyComments)?
            .comments
            .to_owned();

        Self::from_vorbis_comments(comments, "flac")
    }

    fn from_ogg_vorbis(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::open(path)?;
        let mut reader = ogg::reading::PacketReader::new(file);
        let ((_, comments, _), _) = lewton::inside_ogg::read_headers(&mut reader)?;
        let comments = Self::ogg_comment_map(comments.comment_list);

        Self::from_vorbis_comments(comments, "ogg")
    }

    fn from_vorbis_comments(comments: HashMap<String, Vec<String>>, ext: &str) -> Result<Self> {
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

    fn from_m4a(path: impl AsRef<Path>) -> Result<Self> {
        let tag = mp4ameta::Tag::read_from_path(path.as_ref())?;

        let artist = tag
            .album_artist()
            .or_else(|| tag.artist())
            .map(|a| a.to_string());

        let ext = path
            .as_ref()
            .extension()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "m4a".to_string());

        Ok(Metadata {
            artist,
            album: tag.album().map(|a| a.to_owned()),
            disc: tag.disc_number().0.map(|this_disk| this_disk.into()),
            track: tag.track_number().0.map(|this_track| this_track.into()),
            title: tag.title().map(|a| a.to_owned()),
            ext,
        })
    }

    pub fn get_artist(&self) -> Result<String> {
        impl_tag_getter!(self, artist)
    }

    pub fn get_album(&self) -> Result<String> {
        impl_tag_getter!(self, album)
    }

    pub fn get_disc(&self) -> Result<String> {
        impl_tag_getter!(self, disc)
    }

    pub fn get_track(&self) -> Result<String> {
        impl_tag_getter!(self, track)
    }

    pub fn get_title(&self) -> Result<String> {
        impl_tag_getter!(self, title)
    }

    pub fn get_ext(&self) -> String {
        self.ext.clone()
    }
}

#[cfg(test)]
mod tests {
    macro_rules! define_unit_test_for {
        ($ext:ident) => {
            #[cfg(test)]
            mod $ext {
                use $crate::metadata::Metadata;
                use $crate::Error;

                #[test]
                fn complete() {
                    let ext = stringify!($ext);
                    let metadata =
                        Metadata::from_path(format!("test_files/complete.{}", ext)).unwrap();

                    assert_eq!(Ok("Album Artist".into()), metadata.get_artist());
                    assert_eq!(Ok("Album".into()), metadata.get_album());
                    assert_eq!(Ok("1".into()), metadata.get_disc());
                    assert_eq!(Ok("1".into()), metadata.get_track());
                    assert_eq!(Ok("Title".into()), metadata.get_title());
                    assert_eq!(ext.to_string(), metadata.get_ext());
                }

                #[test]
                fn partial() {
                    let ext = stringify!($ext);
                    let metadata =
                        Metadata::from_path(format!("test_files/partial.{}", ext)).unwrap();

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
                    assert_eq!(ext.to_string(), metadata.get_ext());
                }
            }
        };
    }

    define_unit_test_for!(flac);
    define_unit_test_for!(mp3);
    define_unit_test_for!(ogg);
    define_unit_test_for!(m4a);
    define_unit_test_for!(m4p);
}
