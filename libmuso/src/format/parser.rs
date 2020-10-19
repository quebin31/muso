use nom::branch::alt;
use nom::bytes::complete::{tag, take_till1};
use nom::character::complete::{char, digit1};
use nom::combinator::{map, opt};
use nom::multi::many1;
use nom::sequence::{delimited, tuple};
use nom::IResult;

use crate::{Error, Result};

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
        matches!(self, Placeholder::Optional(_))
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

pub(crate) fn parse_format_string(input: &str) -> Result<Vec<BasicComponent>> {
    let (rest, parsed) = components(input).map_err(|_| Error::FailedToParse)?;

    if !rest.is_empty() {
        Err(Error::FailedToParse)
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
