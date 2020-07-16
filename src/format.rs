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

use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::bytes::complete::take_till1;
use nom::character::complete::char;
use nom::character::complete::digit1;
use nom::combinator::map;
use nom::combinator::opt;
use nom::multi::many1;
use nom::sequence::delimited;

use nom::sequence::tuple;
use nom::IResult;

use crate::error::{MusoError, MusoResult};
use crate::metadata::Metadata;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Tag {
    Artist,
    Album,
    Disc { leading: u8 },
    Track { leading: u8 },
    Title,
    Ext,
}

impl From<&str> for Tag {
    fn from(input: &str) -> Self {
        match input {
            "artist" => Tag::Artist,
            "album" => Tag::Album,
            "disc" | "disk" => Tag::Disc { leading: 0 },
            "track" => Tag::Track { leading: 0 },
            "title" => Tag::Title,
            "ext" => Tag::Ext,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Placeholder {
    Required(Tag),
    Optional(Tag),
}

impl Placeholder {
    pub fn is_optional(&self) -> bool {
        match self {
            Placeholder::Optional(_) => true,
            _ => false,
        }
    }

    pub fn is_tag(&self, tag: Tag) -> bool {
        match self {
            Placeholder::Required(other) | Placeholder::Optional(other) => tag == *other,
        }
    }

    pub fn into_tag(self) -> Tag {
        match self {
            Placeholder::Required(tag) | Placeholder::Optional(tag) => tag,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BasicComponent {
    String(String),
    Placeholder(Placeholder),
}

#[derive(Debug, Clone, PartialEq)]
pub enum FsComponent {
    Dir(Vec<BasicComponent>),
    File(Vec<BasicComponent>),
}

#[derive(Debug, Clone)]
pub struct ParsedFormat {
    fs_components: Vec<FsComponent>,
}

impl FromStr for ParsedFormat {
    type Err = MusoError;

    fn from_str(s: &str) -> MusoResult<Self> {
        let basic_components = parse_format_string(s)?;

        let mut fs_components = Vec::new();
        let mut fs_component = Vec::new();

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

        Ok(Self { fs_components })
    }
}

impl ParsedFormat {
    pub fn build_path(&self, metadata: &Metadata, exfat_compat: bool) -> MusoResult<String> {
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
                                    .ok_or_else(|| MusoError::OptionalInDir)?;

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
                        return Err(MusoError::RequiredInFile);
                    }
                }
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

    fn get_from_metadata(metadata: &Metadata, pholder: Placeholder) -> MusoResult<Option<String>> {
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

fn tag_ident(input: &str) -> IResult<&str, &str> {
    alt((
        tag("ext"),
        tag("disc"),
        tag("disk"),
        tag("track"),
        tag("title"),
        tag("album"),
        tag("artist"),
    ))(input)
}

fn tag_leading(input: &str) -> IResult<&str, u8> {
    let (input, output) = opt(tuple((char(':'), digit1)))(input)?;

    Ok((
        input,
        output.map(|(_, n)| n.parse().unwrap()).unwrap_or_else(|| 0),
    ))
}

fn tag_complete(input: &str) -> IResult<&str, Tag> {
    let (input, output) = tag_ident(input)?;

    let (input, tag) = match Tag::from(output) {
        Tag::Disc { .. } => {
            let (input, leading) = tag_leading(input)?;
            (input, Tag::Disc { leading })
        }

        Tag::Track { .. } => {
            let (input, leading) = tag_leading(input)?;
            (input, Tag::Track { leading })
        }

        placeholder => (input, placeholder),
    };

    Ok((input, tag))
}

fn placeholder(input: &str) -> IResult<&str, Placeholder> {
    let (input, placeholder) = tag_complete(input)?;

    let (input, component) = match placeholder {
        p @ Tag::Ext => (input, Placeholder::Required(p)),
        p => {
            let (input, optional) = opt(char('?'))(input)?;
            let placeholder = if optional.is_some() {
                Placeholder::Optional(p)
            } else {
                Placeholder::Required(p)
            };

            (input, placeholder)
        }
    };

    Ok((input, component))
}

fn component(input: &str) -> IResult<&str, BasicComponent> {
    alt((
        map(take_till1(|c: char| c == '{'), |s: &str| {
            BasicComponent::String(s.into())
        }),
        map(delimited(char('{'), placeholder, char('}')), |p| {
            BasicComponent::Placeholder(p)
        }),
    ))(input)
}

fn components(input: &str) -> IResult<&str, Vec<BasicComponent>> {
    many1(component)(input)
}

fn parse_format_string(input: &str) -> MusoResult<Vec<BasicComponent>> {
    let (rest, parsed) = components(input).map_err(|_| MusoError::FailedToParse)?;

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
    fn tag_leading_parse() {
        assert_eq!(tag_leading(":2"), Ok(("", 2)));
        assert_eq!(tag_leading("a:2"), Ok(("a:2", 0)));
        assert_eq!(tag_leading("?}"), Ok(("?}", 0)));
        assert_eq!(tag_leading(":2?}"), Ok(("?}", 2)));
    }

    #[test]
    fn tag_complete_parse() {
        assert_eq!(tag_complete("artist"), Ok(("", Tag::Artist)));
        assert_eq!(tag_complete("disc:2"), Ok(("", Tag::Disc { leading: 2 })));
        assert_eq!(
            tag_complete("track:3?}"),
            Ok(("?}", Tag::Track { leading: 3 }))
        );
        assert_eq!(tag_complete("disk"), Ok(("", Tag::Disc { leading: 0 })));
    }

    #[test]
    fn placeholder_parse() {
        assert_eq!(
            placeholder("artist?"),
            Ok(("", Placeholder::Optional(Tag::Artist)))
        );
        assert_eq!(
            placeholder("album}"),
            Ok(("}", Placeholder::Required(Tag::Album)))
        );
        assert_eq!(
            placeholder("disc:2?"),
            Ok(("", Placeholder::Optional(Tag::Disc { leading: 2 })))
        );
        assert_eq!(
            placeholder("track?}"),
            Ok(("}", Placeholder::Optional(Tag::Track { leading: 0 })))
        );
    }

    #[test]
    fn component_parse() {
        assert_eq!(
            component("foo"),
            Ok(("", BasicComponent::String("foo".into())))
        );
        assert_eq!(
            component("foo{artist?}"),
            Ok(("{artist?}", BasicComponent::String("foo".into())))
        );
        assert_eq!(
            component("{artist}"),
            Ok((
                "",
                BasicComponent::Placeholder(Placeholder::Required(Tag::Artist))
            ))
        );

        assert_eq!(
            component("{track:2}"),
            Ok((
                "",
                BasicComponent::Placeholder(Placeholder::Required(Tag::Track { leading: 2 }))
            ))
        );
    }

    #[test]
    fn components_parse() {
        let expected = vec![
            BasicComponent::Placeholder(Placeholder::Required(Tag::Artist)),
            BasicComponent::String("/".into()),
            BasicComponent::Placeholder(Placeholder::Required(Tag::Album)),
            BasicComponent::String("/".into()),
            BasicComponent::Placeholder(Placeholder::Optional(Tag::Track { leading: 2 })),
            BasicComponent::String(" - ".into()),
            BasicComponent::Placeholder(Placeholder::Required(Tag::Title)),
            BasicComponent::String(".".into()),
            BasicComponent::Placeholder(Placeholder::Required(Tag::Ext)),
        ];

        let parsed = components("{artist}/{album}/{track:2?} - {title}.{ext}");

        assert_eq!(parsed, Ok(("", expected)));
    }

    #[test]
    fn without_placeholders() {
        let expected = vec![BasicComponent::String("hello world".into())];

        let parsed = components("hello world");
        assert_eq!(parsed, Ok(("", expected)));
    }
}
