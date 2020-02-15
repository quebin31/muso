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

use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use infer;

use crate::error::MusoError;

#[derive(Debug)]
pub struct Metadata {
    pub artist: Option<String>,
    pub album: Option<String>,
    pub track: Option<u32>,
    pub title: Option<String>,
    pub ext: String,
}

impl Metadata {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
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

    fn from_id3(path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        let tag = id3::Tag::read_from_path(path)?;

        let artist = if let Some(artist) = tag.album_artist() {
            Some(artist.to_owned())
        } else {
            tag.artist().map(|s| s.to_owned())
        };

        let album = tag.album().map(|s| s.to_owned());
        let track = tag.track();
        let title = tag.title().map(|s| s.to_owned());

        Ok(Metadata {
            artist,
            album,
            track,
            title,
            ext: "mp3".to_owned(),
        })
    }

    fn from_vorbis(path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
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
            track,
            title,
            ext: "flac".to_owned(),
        })
    }

    pub fn build_path(&self, format: &str, exfat_compat: bool) -> Result<String, MusoError> {
        let path = format
            .replace(
                "{artist}",
                &replace(
                    self.artist
                        .as_ref()
                        .ok_or_else(|| MusoError::MissingTagProperty("artist".to_owned()))?,
                    exfat_compat,
                ),
            )
            .replace(
                "{album}",
                &replace(
                    self.album
                        .as_ref()
                        .ok_or_else(|| MusoError::MissingTagProperty("album".to_owned()))?,
                    exfat_compat,
                ),
            )
            .replace(
                "{track}",
                &replace(
                    &self
                        .track
                        .as_ref()
                        .ok_or_else(|| MusoError::MissingTagProperty("track".to_owned()))?
                        .to_string(),
                    exfat_compat,
                ),
            )
            .replace(
                "{title}",
                &replace(
                    self.title
                        .as_ref()
                        .ok_or_else(|| MusoError::MissingTagProperty("title".to_owned()))?,
                    exfat_compat,
                ),
            )
            .replace("{ext}", &self.ext);

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
    fn correct_path() {
        let metadata = Metadata {
            artist: Some("Cage The Elephant".to_owned()),
            album: Some("Social Cues".to_owned()),
            track: Some(1),
            title: Some("Social Cues".to_owned()),
            ext: "flac".to_owned(),
        };

        assert_eq!(
            Ok("Cage The Elephant/Social Cues/1 - Social Cues.flac".into()),
            metadata.build_path("{artist}/{album}/{track} - {title}.{ext}", false)
        );
    }

    #[test]
    fn incorrect_path() {
        let metadata = Metadata {
            artist: Some("Cage The Elephant".to_owned()),
            album: None,
            track: Some(1),
            title: Some("Social Cues".to_owned()),
            ext: "flac".to_owned(),
        };

        assert_eq!(
            Err(MusoError::MissingTagProperty("album".to_owned())),
            metadata.build_path("{artist}/{album}/{track} - {title}.{ext}", false)
        );
    }
}
