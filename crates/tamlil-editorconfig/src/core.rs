//! The core is responsible to parse EditorConfig files, and produce headers and
//! pairs.

use crate::glob::{self, Glob};

/// Characters which converts a line to a comment (thus discardable) when
/// preceding it.
const COMMENT_STARTERS: &[char] = &['#', ';'];

pub(crate) enum Error {
    /// Encountered an invalid glob expression.
    Glob(glob::Error),
    EmptyKey,

    InvalidLine,
}

pub(crate) struct Document<'src> {
    pub(crate) preamble: Vec<Pair<'src>>,
    pub(crate) sections: Vec<Section<'src>>,
}

impl<'src> Document<'src> {
    pub(crate) fn parse(source: &'src str) -> Result<Self, Error> {
        let lines = parse_file(source);
        let mut doc = Self { preamble: vec![], sections: vec![] };

        for line in lines {
            match line? {
                Line::Header(header) => {
                    doc.sections.push(Section { header, pairs: vec![] });
                }
                Line::Pair(pair) => {
                    let current_pairs = doc
                        .sections
                        .last_mut()
                        .map(|s| &mut s.pairs)
                        .unwrap_or(&mut doc.preamble);
                    current_pairs.push(pair);
                }
            }
        }

        Ok(doc)
    }
}

pub(crate) struct Section<'src> {
    pub(crate) header: Header,
    pub(crate) pairs: Vec<Pair<'src>>,
}

pub(crate) struct Header {
    pub(crate) glob: Glob,
}

pub(crate) struct Pair<'src> {
    pub(crate) key: &'src str,
    pub(crate) value: &'src str,
}

enum Line<'src> {
    Header(Header),
    Pair(Pair<'src>),
}

impl<'src> Line<'src> {
    fn header(glob: Glob) -> Self {
        Self::Header(Header { glob })
    }

    fn pair(key: &'src str, value: &'src str) -> Self {
        Self::Pair(Pair { key, value })
    }
}

fn parse_file(source: &str) -> impl Iterator<Item = Result<Line<'_>, Error>> {
    source
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .filter(|l| !l.starts_with(COMMENT_STARTERS))
        .map(parse_line)
}

fn parse_line(l: &str) -> Result<Line<'_>, Error> {
    parse_header(l).or_else(|| parse_pair(l)).unwrap_or(Err(Error::InvalidLine))
}

fn parse_header(l: &str) -> Option<Result<Line<'_>, Error>> {
    let glob = l.strip_prefix('[').and_then(|l| l.strip_suffix(']'))?;
    Some(Glob::new(glob).map(Line::header).map_err(Error::Glob))
}

fn parse_pair(l: &str) -> Option<Result<Line<'_>, Error>> {
    let (key, value) = l.split_once('=')?;
    let key = key.trim();
    if key.is_empty() {
        Some(Err(Error::InvalidLine))
    } else {
        Some(Ok(Line::pair(key, value.trim())))
    }
}
