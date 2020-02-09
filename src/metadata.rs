use std::error::Error as StdError;
use std::path::{Path, PathBuf};

use infer;

use crate::error::Error;

#[derive(Debug)]
pub struct Metadata {
    pub artist: String,
    pub album: String,
    pub track: u32,
    pub title: String,
    pub ext: String,
}

impl Metadata {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, Box<dyn StdError>> {
        let info = infer::Infer::new();
        let file = info.get_from_path(&path)?.ok_or(Error::CantInferMime)?;
        match file.mime.as_str() {
            "audio/x-flac" => Metadata::from_vorbis(&path),
            "audio/mpeg" => Metadata::from_id3(&path),
            _ => Err(Error::NotSupported.into()),
        }
    }

    fn from_id3(path: impl AsRef<Path>) -> Result<Self, Box<dyn StdError>> {
        let tag = id3::Tag::read_from_path(path)?;

        let artist = if let Some(artist) = tag.album_artist() {
            artist
        } else {
            tag.artist().unwrap_or("")
        }
        .replace("/", "_");

        let album = tag.album().unwrap_or("").replace("/", "_");
        let track = tag.track().unwrap_or(0);
        let title = tag.title().unwrap_or("").replace("/", "_");

        Ok(Metadata {
            artist,
            album,
            track,
            title,
            ext: "mp3".to_owned(),
        })
    }

    fn from_vorbis(path: impl AsRef<Path>) -> Result<Self, Box<dyn StdError>> {
        let tag = metaflac::Tag::read_from_path(path)?;
        let comments = &tag.vorbis_comments().ok_or(Error::EmptyComments)?.comments;
        let empty = "".to_owned();

        let artist = if let Some(artist) = comments.get("ALBUMARTIST").and_then(|a| a.get(0)) {
            artist
        } else {
            comments
                .get("ARTIST")
                .map_or("", |a| a.get(0).unwrap_or(&empty))
        }
        .replace("/", "_");

        let album = comments
            .get("ALBUM")
            .map_or("", |a| a.get(0).unwrap_or(&empty))
            .replace("/", "_");

        let track = comments
            .get("TRACKNUMBER")
            .map_or("", |t| t.get(0).unwrap_or(&empty))
            .parse::<u32>()
            .unwrap_or(0);

        let title = comments
            .get("TITLE")
            .map_or("", |t| t.get(0).unwrap_or(&empty))
            .replace("/", "_");

        Ok(Metadata {
            artist,
            album,
            track,
            title,
            ext: "flac".to_owned(),
        })
    }

    pub fn build_path(&self, format: &str) -> Result<PathBuf, Error> {
        if format.contains("{artist}") && self.artist.is_empty() {
            Err(Error::MissingTagProperty("artist".to_owned()))
        } else if format.contains("{album}") && self.album.is_empty() {
            Err(Error::MissingTagProperty("album".to_owned()))
        } else if format.contains("{track}") && self.track == 0 {
            Err(Error::MissingTagProperty("track".to_owned()))
        } else if format.contains("{title}") && self.title.is_empty() {
            Err(Error::MissingTagProperty("title".to_owned()))
        } else if format.contains("{ext}") && self.ext.is_empty() {
            Err(Error::MissingTagProperty("ext".to_owned()))
        } else {
            let path = format
                .replace("{artist}", &self.artist)
                .replace("{album}", &self.album)
                .replace("{track}", &self.track.to_string())
                .replace("{title}", &self.title)
                .replace("{ext}", &self.ext);

            Ok(path.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correct_path() {
        let metadata = Metadata {
            artist: "Cage The Elephant".to_owned(),
            album: "Social Cues".to_owned(),
            track: 1,
            title: "Social Cues".to_owned(),
            ext: "flac".to_owned(),
        };

        assert_eq!(
            Some("Cage The Elephant/Social Cues/1 - Social Cues.flac".into()),
            metadata.build_path("{artist}/{album}/{track} - {title}.{ext}")
        );
    }

    #[test]
    fn incorrect_path() {
        let metadata = Metadata {
            artist: "Cage The Elephant".to_owned(),
            album: "".to_owned(),
            track: 1,
            title: "Social Cues".to_owned(),
            ext: "flac".to_owned(),
        };

        assert_eq!(
            None,
            metadata.build_path("{artist}/{album}/{track} - {title}.{ext}")
        );
    }
}
