use std::str::FromStr;

use muso::error::{AnyResult, MusoError};
use muso::format::ParsedFormat;
use muso::metadata::Metadata;

fn complete_with_ok_format(ext: &str) -> AnyResult<()> {
    let metadata = Metadata::from_path(format!("test_files/complete.{}", ext))?;
    let format = ParsedFormat::from_str("{artist}/{album}/{disc}.{track} - {title}.{ext}")?;

    assert_eq! {
        Ok(format!("Album Artist/Album/1.1 - Title.{}", ext)),
        format.build_path(&metadata, false)
    };

    Ok(())
}

fn partial_with_ok_format(ext: &str) -> AnyResult<()> {
    let metadata = Metadata::from_path(format!("test_files/partial.{}", ext))?;
    let format = ParsedFormat::from_str("{artist}/{disc}.{track} - {title}.{ext}")?;

    assert_eq! {
        Ok(format!("Artist/1.1 - Title.{}", ext)),
        format.build_path(&metadata, false)
    };

    Ok(())
}

fn partial_with_bad_format(ext: &str) -> AnyResult<()> {
    let metadata = Metadata::from_path(format!("test_files/partial.{}", ext))?;
    let format = ParsedFormat::from_str("{artist}/{album}")?;

    assert_eq! {
        Err(MusoError::MissingTag{ tag: "album".into() }),
        format.build_path(&metadata, false)
    };

    Ok(())
}

#[test]
fn complete_flac_with_ok_format() -> AnyResult<()> {
    complete_with_ok_format("flac")
}

#[test]
fn partial_flac_with_ok_format() -> AnyResult<()> {
    partial_with_ok_format("flac")
}

#[test]
fn partial_flac_with_bad_format() -> AnyResult<()> {
    partial_with_bad_format("flac")
}

#[test]
fn complete_mp3_with_ok_format() -> AnyResult<()> {
    complete_with_ok_format("mp3")
}

#[test]
fn partial_mp3_with_ok_format() -> AnyResult<()> {
    partial_with_ok_format("mp3")
}

#[test]
fn partial_mp3_with_bad_format() -> AnyResult<()> {
    partial_with_bad_format("mp3")
}

#[test]
fn complete_ogg_with_ok_format() -> AnyResult<()> {
    complete_with_ok_format("ogg")
}

#[test]
fn partial_ogg_with_ok_format() -> AnyResult<()> {
    partial_with_ok_format("ogg")
}

#[test]
fn partial_ogg_with_bad_format() -> AnyResult<()> {
    partial_with_bad_format("ogg")
}
