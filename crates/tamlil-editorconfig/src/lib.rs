use std::str::FromStr;

use crate::glob::Glob;
use crate::property::{Charset, EndOfLine, IndentStyle};

mod core;
mod glob;
#[macro_use]
mod property;

/// Version which this implementation complies to.
pub const EDITORCONFIG_VERSION: (u32, u32, u32) = (0, 17, 2);

/// The only valid name for an EditorConfig file.
const FILE_NAME: &str = ".editorconfig";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Error {
    Parse,
}

struct Document {
    preamble: Preamble,
    sections: Vec<Section>,
}

impl FromStr for Document {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let doc = core::Document::parse(s).map_err(|_| Error::Parse)?;
        Ok(Self::from(doc))
    }
}

impl From<core::Document<'_>> for Document {
    fn from(doc: core::Document) -> Self {
        let preamble = Preamble::from_pairs(&doc.preamble);
        let sections = doc.sections.into_iter().map(Section::from).collect();

        Self { preamble, sections }
    }
}

struct Preamble {
    root: Option<bool>,
}

impl Preamble {
    fn from_pairs(pairs: &[core::Pair]) -> Self {
        let root = pairs
            .iter()
            .filter(|pair| pair.key.eq_ignore_ascii_case("root"))
            .filter_map(|p| MaybeUnset::parse(p.value, parse_bool))
            .next_back()
            .and_then(MaybeUnset::into_value);

        Self { root }
    }
}

enum MaybeUnset<T> {
    Value(T),
    Unset,
}

impl<T> MaybeUnset<T> {
    fn parse<F>(s: &str, f: F) -> Option<Self>
    where
        F: FnOnce(&str) -> Option<T>,
    {
        if s.eq_ignore_ascii_case("unset") {
            Some(Self::Unset)
        } else {
            (f)(s).map(Self::Value)
        }
    }

    fn into_value(self) -> Option<T> {
        match self {
            MaybeUnset::Value(value) => Some(value),
            MaybeUnset::Unset => None,
        }
    }
}

fn parse_bool(s: &str) -> Option<bool> {
    case_insensitive_map! { s;
        "true" => true,
        "false" => false,
    }
}

fn parse_u32(s: &str) -> Option<u32> {
    s.parse().ok()
}

struct Section {
    header: Glob,
    properties: Properties,
}

impl From<core::Section<'_>> for Section {
    fn from(section: core::Section<'_>) -> Self {
        let header = section.header.glob;
        let properties = Properties::from_pairs(&section.pairs);

        Self { header, properties }
    }
}

struct Properties {
    indent_style: Option<IndentStyle>,
    indent_size: Option<u32>,
    tab_width: Option<u32>,
    end_of_line: Option<EndOfLine>,
    charset: Option<Charset>,
    spelling_language: Option<String>,
    trim_trailing_whitespace: Option<bool>,
    insert_final_newline: Option<bool>,
}

impl Properties {
    fn empty() -> Self {
        Self {
            indent_style: None,
            indent_size: None,
            tab_width: None,
            end_of_line: None,
            charset: None,
            spelling_language: None,
            trim_trailing_whitespace: None,
            insert_final_newline: None,
        }
    }

    fn from_pairs(pairs: &[core::Pair]) -> Self {
        macro_rules! apply_pair {
            ($props:expr, $key:expr, $value:expr; $( $name:ident => $parse:expr),* $(,)? ) => {
                match $key {
                    $(
                        s if s.eq_ignore_ascii_case(stringify!($name)) => {
                            if let Some(mu) = MaybeUnset::parse($value, $parse) {
                                $props.$name = mu.into_value();
                            }
                        }
                    )*
                    _ => {},
                }
            };
        }

        let mut props = Self::empty();

        for pair in pairs {
            apply_pair! {
                props, pair.key, pair.value;

                indent_style => IndentStyle::parse,
                indent_size => parse_u32,
                tab_width => parse_u32,
                end_of_line => EndOfLine::parse,
                charset => Charset::parse,
                spelling_language => |s| Some(s.to_owned()),
                trim_trailing_whitespace => parse_bool,
                insert_final_newline => parse_bool,
            }
        }

        props
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Document {
        s.parse().unwrap()
    }

    #[test]
    fn test_basic_preamble_and_section() {
        let input = r#"
            root = true

            [*.rs]
            indent_style = space
            indent_size = 4
            end_of_line = lf
        "#;

        let doc = parse(input);
        assert_eq!(doc.preamble.root, Some(true));
        assert_eq!(doc.sections.len(), 1);

        let props = &doc.sections[0].properties;
        assert_eq!(props.indent_style.unwrap(), IndentStyle::Space);
        assert_eq!(props.indent_size.unwrap(), 4);
        assert_eq!(props.end_of_line.unwrap(), EndOfLine::Lf);
    }

    #[test]
    fn test_case_insensitive_keys_and_values() {
        let input = r#"
            ROOT = TRUE

            [*.rs]
            InDent_Style = SPaCe
        "#;

        let doc = parse(input);
        assert!(doc.preamble.root.unwrap());
        let props = &doc.sections[0].properties;
        assert_eq!(props.indent_style.unwrap(), IndentStyle::Space);
    }

    #[test]
    fn test_unset_property() {
        let input = r#"
            [*.rs]
            indent_size = 2
            indent_size = unset
        "#;

        let doc = parse(input);
        let props = &doc.sections[0].properties;
        assert_eq!(props.indent_size, None);
    }

    #[test]
    fn test_property_override() {
        let input = r#"
            [*.rs]
            indent_size = 2
            indent_size = 4
        "#;

        let doc = parse(input);
        let props = &doc.sections[0].properties;
        assert_eq!(props.indent_size.unwrap(), 4);
    }

    #[test]
    fn test_multiple_sections() {
        let input = r#"
            [*.rs]
            indent_style = space

            [*.py]
            indent_style = tab
        "#;

        let doc = parse(input);
        assert_eq!(doc.sections.len(), 2);
        assert_eq!(
            doc.sections[0].properties.indent_style.unwrap(),
            IndentStyle::Space
        );
        assert_eq!(
            doc.sections[1].properties.indent_style.unwrap(),
            IndentStyle::Tab
        );
    }

    #[test]
    fn test_spelling_language_and_charset() {
        let input = r#"
            [*.md]
            spelling_language = en-GB
            charset = utf-8
        "#;

        let doc = parse(input);
        let props = &doc.sections[0].properties;
        assert_eq!(props.spelling_language.as_ref().unwrap(), "en-GB");
        assert_eq!(props.charset.unwrap(), Charset::Utf8);
    }

    #[test]
    fn test_invalid_property_is_ignored() {
        let input = r#"
            [*.rs]
            some_unknown_property = value
        "#;

        let doc = parse(input);
        let props = &doc.sections[0].properties;

        assert!(props.indent_size.is_none());
        assert!(props.indent_style.is_none());
        assert!(props.spelling_language.is_none());
    }
}
