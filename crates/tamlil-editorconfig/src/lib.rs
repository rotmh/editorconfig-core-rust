#![feature(trim_prefix_suffix, trait_alias)]

mod glob;
mod version;

use std::collections::HashMap;
use std::convert::identity;
use std::fs::File;
use std::io::{self, BufRead as _, BufReader};
use std::path::Path;

use crate::glob::Glob;
pub use crate::version::Version;

pub const MAX_VERSION: Version = Version { major: 0, minor: 17, patch: 2 };
pub const DEFAULT_FILE_NAME: &str = ".editorconfig";

const DEFAULT_ALLOW_UNSET: bool = true;

const UNSET_VALUE: &str = "unset";

#[derive(Debug)]
pub enum Error {
    Parse,
    InvalidPath,
    Io(io::Error),
}

pub struct Options<'a> {
    pub file_name: &'a str,
    pub allow_unset: bool,
    pub version: Version,
}

impl<'a> Default for Options<'a> {
    fn default() -> Self {
        Self {
            file_name: DEFAULT_FILE_NAME,
            allow_unset: DEFAULT_ALLOW_UNSET,
            version: MAX_VERSION,
        }
    }
}

pub trait Property {
    type Error;

    const KEYS: &[&str];

    fn parse(value: &str) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

pub struct Properties(HashMap<String, String>);

impl Properties {
    /// Retreives the properties for the file at `path`.
    ///
    /// Uses the default options, which are `".editorconfig"` as the filename
    /// for EditorConfig files, and recognizing `"unset"` values
    /// (case-insensitive) - which leads to discarding properties with `"unset"`
    /// values.
    ///
    /// Note: `path` doesn't have to exist.
    pub fn new<P>(path: P) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        Self::new_with_options(path, Options::default())
    }

    pub fn new_with_options<P>(path: P, options: Options) -> Result<Self, Error>
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

        Ok(Self(properties))
    }

    pub fn get<P>(&self) -> Option<Result<P, P::Error>>
    where
        P: Property,
    {
        let value = P::KEYS.iter().find_map(|&key| self.0.get(key))?;
        Some(P::parse(value))
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }
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
        let l = line.trim_suffix('\n').trim_suffix('\r').trim();
        if l.starts_with(COMMENT) {
            // We ignore comment lines.
        } else if let Some(is_match) =
            parse_section(normalized_file_path, &normalized_ec_dir, l)?
        {
            section_matches_file = Some(is_match);
        } else if section_matches_file.is_some_and(identity)
            && let Some((key, value)) = parse_pair(l)
        {
            if options.allow_unset && value.eq_ignore_ascii_case(UNSET_VALUE) {
                properties.remove(&key.to_lowercase());
            } else {
                properties.insert(key.to_lowercase(), value.to_lowercase());
            }
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
