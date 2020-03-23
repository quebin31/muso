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

use std::str::FromStr;

use nom::character::complete::{anychar, digit1};
use nom::IResult;
use nom::{alt, char, complete, delimited, many0, separated_pair, tag, take_while};

use crate::error::{MusoError, MusoResult};
use crate::metadata::Metadata;

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

#[derive(Debug, Clone)]
pub struct ParsedFormat {
    components: Vec<Component>,
}

impl FromStr for ParsedFormat {
    type Err = MusoError;
    fn from_str(s: &str) -> MusoResult<Self> {
        parse_format_string(s)
    }
}

impl ParsedFormat {
    pub fn build_path(&self, metadata: &Metadata, exfat_compat: bool) -> MusoResult<String> {
        let mut path = String::with_capacity(128);

        for component in &self.components {
            match component {
                Component::String(s) => path.push_str(&s),
                Component::Placeholder(p) => {
                    let value = match p {
                        Placeholder::Artist => metadata.get_artist()?,
                        Placeholder::Album => metadata.get_album()?,
                        Placeholder::Disc { leading } => {
                            let value = metadata.get_disc()?;
                            Self::add_leading_zeros(value, *leading)
                        }
                        Placeholder::Track { leading } => {
                            let value = metadata.get_track()?;
                            Self::add_leading_zeros(value, *leading)
                        }
                        Placeholder::Title => metadata.get_title()?,
                        Placeholder::Ext => metadata.get_ext(),
                    };

                    path.push_str(&Self::replace(value, exfat_compat));
                }

                _ => unreachable!(),
            }
        }

        Ok(path)
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
}

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

fn parse_inner(input: &str) -> IResult<&str, Vec<Component>> {
    let (input, components) = many0!(input, component)?;

    let mut parsed = Vec::new();
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

    if !free.is_empty() {
        parsed.push(Component::String(free));
    }

    Ok((input, parsed))
}

fn parse_format_string(input: &str) -> MusoResult<ParsedFormat> {
    let (rest, parsed) = parse_inner(input).map_err(|_| MusoError::FailedToParse)?;

    if !rest.is_empty() {
        Err(MusoError::FailedToParse)
    } else {
        Ok(ParsedFormat { components: parsed })
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
        assert_eq!(
            component("{artist}"),
            Ok(("", Component::Placeholder(Placeholder::Artist)))
        );

        assert_eq!(
            component("{track:2}"),
            Ok((
                "",
                Component::Placeholder(Placeholder::Track { leading: 2 })
            ))
        );
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
