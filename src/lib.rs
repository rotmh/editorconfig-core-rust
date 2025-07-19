//! An EditorConfig Core passing all the [editorconfig-core-test] tests.
//!
//! # Examples
//!
//! ```no_run
//! use editorconfig_core::properties;
//!
//! // Let's define the property we want to extract.
//!
//! enum EndOfLine { Cr, Crlf, Lf }
//!
//! impl EndOfLine {
//!     const KEY: &str = "end_of_line";
//!
//!     fn from_str<S: AsRef<str>>(s: S) -> Option<Self> {
//!         match s.as_ref() {
//!             "cr" => Some(Self::Cr),
//!             "crlf" => Some(Self::Crlf),
//!             "lf" => Some(Self::Lf),
//!             _ => None,
//!         }
//!     }
//! }
//!
//! // Now, fetch the properties for our file.
//!
//! // Must be a full, normalized, valid unicode path.
//! let path = "/home/myself/README.md";
//!
//! let mut properties = properties(path).unwrap();
//!
//! // Discard properties that was unset.
//! properties.retain(|_key, value| !value.eq_ignore_ascii_case("unset"));
//!
//! // Extract the property.
//! let eof = properties.get(EndOfLine::KEY).and_then(EndOfLine::from_str);
//! ```
//!
//! # Notes
//!
//! - All the keys are already lowercased via `str::to_lowercase`.
//! - The values are kept in their original form, except for the values of the ["Supported"](https://editorconfig.org/#supported-properties)
//!   properties.
//!
//! # CLI
//!
//! This package contains a binary crate as well as the library. This binary
//! contains an EditorConfig CLI which was created for testing purposes, as
//! [editorconfig-core-test] operates on CLIs.
//!
//! Although it was created for testing, you can use it in your project for
//! extracting properties of a path from the shell.
//!
//! [editorconfig-core-test]: https://github.com/editorconfig/editorconfig-core-test

mod glob;
mod version;

use std::collections::HashMap;
use std::convert::identity;
use std::fs::File;
use std::io::{self, BufRead as _, BufReader};
use std::path::Path;

use crate::glob::Glob;
pub use crate::version::Version;

/// Max. supported EditorConfig version.
pub const MAX_VERSION: Version = Version { major: 0, minor: 17, patch: 2 };

#[derive(Debug)]
pub enum Error {
    Parse,
    InvalidPath,
    Io(io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options<'a> {
    /// Another name for EditorConfig files (defaults to ".editorconfig").
    pub file_name: &'a str,
    /// EditorConfig version to use (defaults to [`MAX_VERSION`]).
    pub version: Version,
}

impl<'a> Default for Options<'a> {
    fn default() -> Self {
        Self { file_name: ".editorconfig", version: MAX_VERSION }
    }
}

/// All the keys are lowercased, values are kept in their original form, except
/// for the values of "Supported" properties.
pub type Properties = HashMap<String, String>;

/// Retreives the properties for the file at `path`.
///
/// Note: `path` doesn't have to exist.
pub fn properties<P>(path: P) -> Result<Properties, Error>
where
    P: AsRef<Path>,
{
    properties_with_options(path, Options::default())
}

pub fn properties_with_options<P>(
    path: P,
    options: Options,
) -> Result<Properties, Error>
where
    P: AsRef<Path>,
{
    let normalized_path = normalize_path(path.as_ref())?;
    let mut properties = HashMap::new();

    let ancestors: Vec<_> = path.as_ref().ancestors().skip(1).collect();

    for dir in ancestors.iter().rev() {
        parse_dir(dir, &normalized_path, &options, &mut properties)?;
    }

    process_properties(&mut properties, &options);

    properties.retain(|key, _value| key != "unset");

    Ok(properties)
}

/// Process and modify the properties to adhere to the specification at the
/// version in `options`.
fn process_properties(
    properties: &mut HashMap<String, String>,
    options: &Options,
) {
    // TODO: explain what's happening here.

    const V0_9_0: Version = Version { major: 0, minor: 9, patch: 0 };

    const INDENT_STYLE: &str = "indent_style";
    const INDENT_SIZE: &str = "indent_size";
    const TAB_WIDTH: &str = "tab_width";
    const TAB: &str = "tab";

    if options.version.cmp(&V0_9_0).is_ge() {
        if properties.get(INDENT_STYLE).is_some_and(|v| v == TAB)
            && !properties.contains_key(INDENT_SIZE)
        {
            properties.insert(INDENT_SIZE.to_owned(), TAB.to_owned());
        }

        if properties.get(INDENT_SIZE).is_some_and(|v| v == TAB)
            && let Some(tab_width) = properties.get(TAB_WIDTH)
        {
            properties.insert(INDENT_SIZE.to_owned(), tab_width.to_owned());
        }
    }

    if let Some(indent_size) = properties.get(INDENT_SIZE)
        && !properties.contains_key(TAB_WIDTH)
        && (options.version.cmp(&V0_9_0).is_lt() || indent_size != TAB)
    {
        properties.insert(TAB_WIDTH.to_owned(), indent_size.to_owned());
    }
}

fn parse_dir(
    ec_dir: &Path,
    normalized_file_path: &str,
    options: &Options,
    properties: &mut HashMap<String, String>,
) -> Result<(), Error> {
    const COMMENT: &[char] = &['#', ';'];

    let ec_file_path = ec_dir.join(options.file_name);
    let ec_file = match File::open(ec_file_path) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            // The EditorConfig file doesn't have to exist at any of the dirs.
            return Ok(());
        }
        Err(e) => return Err(Error::Io(e)),
    };

    let normalized_ec_dir = normalize_path(ec_dir)?;

    let mut reader = BufReader::new(ec_file);

    let mut line = String::new();

    let mut section_matches_file = None;

    while reader.read_line(&mut line).map_err(Error::Io)? != 0 {
        let l = line
            .strip_suffix('\n')
            .unwrap_or(&line)
            .strip_suffix('\r')
            .unwrap_or(&line)
            .trim();

        if l.starts_with(COMMENT) {
            // We ignore comment lines.
        } else if let Some(is_match) =
            parse_section(normalized_file_path, &normalized_ec_dir, l)?
        {
            section_matches_file = Some(is_match);
        } else if section_matches_file.is_some_and(identity)
            && let Some((key, value)) = parse_pair(l)
        {
            insert_pair(properties, key, value);
        } else if section_matches_file.is_none()
            && let Some((key, value)) = parse_pair(l)
            && key.eq_ignore_ascii_case("root")
            && value.eq_ignore_ascii_case("true")
        {
            // We walk from the root to the directory of the target file, so if
            // an EditorConfig file is a root, it means that all the
            // EditorConfig files "below" it should be discarded.
            properties.clear();
        }

        line.clear();
    }

    Ok(())
}

fn insert_pair(
    properties: &mut HashMap<String, String>,
    key: &str,
    value: &str,
) {
    const SPECIAL_KEYS: &[&str] = &[
        "end_of_line",
        "indent_style",
        "indent_size",
        "insert_final_newline",
        "trim_trailing_whitespace",
        "charset",
    ];

    let key = key.to_lowercase();
    let value = if SPECIAL_KEYS.contains(&key.as_str()) {
        value.to_lowercase()
    } else {
        value.to_owned()
    };

    properties.insert(key, value);
}

fn parse_section(
    normalized_file_path: &str,
    normalized_ec_dir: &str,
    line: &str,
) -> Result<Option<bool>, Error> {
    let Some(pattern) =
        line.strip_prefix('[').and_then(|l| l.strip_suffix(']'))
    else {
        return Ok(None);
    };
    let glob =
        Glob::new(normalized_ec_dir, pattern).map_err(|_| Error::Parse)?;
    Ok(Some(glob.is_match(normalized_file_path)))
}

fn parse_pair(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once('=')?;
    let (key, value) = (key.trim(), value.trim());
    (!key.is_empty()).then_some((key, value))
}

fn normalize_path(path: &Path) -> Result<String, Error> {
    let path = path.to_str().ok_or(Error::InvalidPath)?;
    Ok(if cfg!(windows) { path.replace('\\', "/") } else { path.to_owned() })
}
