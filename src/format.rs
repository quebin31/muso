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

use nom::character::complete::digit1;
use nom::IResult;
use nom::{alt, char, complete, delimited, many0, separated_pair, tag, take_while};

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
        take_while!(|c| c != '{') => { |s: &str| Component::String(s.into()) }
    }
}

pub fn parse_format_string(input: &str) -> IResult<&str, ParsedFormat> {
    many0!(input, component)
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
        assert_eq!(component("foo"), Ok(("", Component::String("foo".into()))));
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

        let parsed = parse_format_string("{artist}/{album}/{track} - {title}.{ext}");

        assert_eq!(parsed, Ok(("", expected)));
    }

    #[test]
    fn leading_zeros_parsing() {
        let expected = vec![
            Component::Placeholder(Placeholder::Disc { leading: 2 }),
            Component::String(" - ".into()),
            Component::Placeholder(Placeholder::Track { leading: 2 }),
        ];

        let parsed = parse_format_string("{disc:2} - {track:2}");

        assert_eq!(parsed, Ok(("", expected)));
    }
}
