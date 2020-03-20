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

use nom::character::complete::{anychar, digit1};
use nom::IResult;
use nom::{alt, char, complete, delimited, many0, separated_pair, tag, take_while};

use crate::error::MusoError;

#[derive(Debug, Clone, PartialEq)]
pub enum Placeholder {
    Artist,
    Album,
    Disc { leading: u8 },
    Track { leading: u8 },
    Title,
    Ext,
}

impl From<&str> for Placeholder {
    fn from(input: &str) -> Self {
        match input {
            "artist" => Placeholder::Artist,
            "album" => Placeholder::Album,
            "disc" => Placeholder::Disc { leading: 0 },
            "track" => Placeholder::Track { leading: 0 },
            "title" => Placeholder::Title,
            "ext" => Placeholder::Ext,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Component {
    Char(char),
    String(String),
    Placeholder(Placeholder),
}

pub type ParsedFormat = Vec<Component>;

fn ident(input: &str) -> IResult<&str, &str> {
    alt! {
        input,
        tag!("artist") |
        tag!("album")  |
        tag!("disc")   |
        tag!("track")  |
        tag!("title")  |
        tag!("ext")
    }
}

fn placeholder<'a>(input: &'a str) -> IResult<&'a str, Placeholder> {
    let (input, output) = delimited! {
        input,
        char!('{'),
        take_while!(|c: char| c.is_alphanumeric() || c == ':'),
        char!('}')
    }?;

    let (_, output) = alt! {
        output,
        complete!(separated_pair!(ident, char!(':'), digit1)) => { |r: (&'a str, &'a str)| (r.0, Some(r.1))} |
        ident => { |r: &'a str| (r, None) }
    }?;

    let leading = output.1.unwrap_or_else(|| "0").parse().unwrap();

    let placeholder = match Placeholder::from(output.0) {
        Placeholder::Disc { .. } => Placeholder::Disc { leading },
        Placeholder::Track { .. } => Placeholder::Track { leading },
        placeholder => placeholder,
    };

    Ok((input, placeholder))
}

fn component(input: &str) -> IResult<&str, Component> {
    alt! {
        input,
        complete!(placeholder) => { |p| Component::Placeholder(p) } |
        anychar => { |c| Component::Char(c) }
    }
}

fn parse_inner(input: &str) -> IResult<&str, ParsedFormat> {
    let (input, components) = many0!(input, component)?;

    let mut parsed = ParsedFormat::new();
    let mut free = String::with_capacity(10);
    for component in components {
        match component {
            Component::Char(c) => free.push(c),
            Component::Placeholder(p) => {
                if !free.is_empty() {
                    parsed.push(Component::String(free.clone()));
                    free.clear();
                }

                parsed.push(Component::Placeholder(p.clone()));
            }
            _ => unreachable!(),
        }
    }

    Ok((input, parsed))
}

pub fn parse_format_string(input: &str) -> Result<ParsedFormat, MusoError> {
    let (rest, parsed) = parse_inner(input).map_err(|_| MusoError::FailedToParse)?;

    if !rest.is_empty() {
        Err(MusoError::FailedToParse)
    } else {
        Ok(parsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_parse() {
        assert_eq!(placeholder("{artist}"), Ok(("", Placeholder::Artist)));
        assert_eq!(
            placeholder("{track}"),
            Ok(("", Placeholder::Track { leading: 0 }))
        );
        assert_eq!(
            placeholder("{track:2}"),
            Ok(("", Placeholder::Track { leading: 2 }))
        );
    }

    #[test]
    fn component_parse() {
        assert_eq!(component("foo"), Ok(("oo", Component::Char('f'))));
    }

    #[test]
    fn basic_format_parsing() {
        let expected = vec![
            Component::Placeholder(Placeholder::Artist),
            Component::String("/".into()),
            Component::Placeholder(Placeholder::Album),
            Component::String("/".into()),
            Component::Placeholder(Placeholder::Track { leading: 0 }),
            Component::String(" - ".into()),
            Component::Placeholder(Placeholder::Title),
            Component::String(".".into()),
            Component::Placeholder(Placeholder::Ext),
        ];

        let parsed = parse_inner("{artist}/{album}/{track} - {title}.{ext}");

        assert_eq!(parsed, Ok(("", expected)));
    }

    #[test]
    fn leading_zeros_parsing() {
        let expected = vec![
            Component::Placeholder(Placeholder::Disc { leading: 2 }),
            Component::String(" - ".into()),
            Component::Placeholder(Placeholder::Track { leading: 2 }),
        ];

        let parsed = parse_inner("{disc:2} - {track:2}");

        assert_eq!(parsed, Ok(("", expected)));
    }

    #[test]
    fn without_placeholders() {
        let expected = vec![Component::String("hello world".into())];

        let parsed = parse_inner("hello world");
        assert_eq!(parsed, Ok(("", expected)));
    }
}
