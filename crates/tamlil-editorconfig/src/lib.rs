//! Implemented referencing the 0.17.2 version of the spec.

mod core;
mod glob;

/// Version which this implementation complies to.
pub const EDITORCONFIG_VERSION: (u32, u32, u32) = (0, 17, 2);

/// The only valid name for an EditorConfig file.
const FILE_NAME: &str = ".editorconfig";

/// For any pair, a value of `"unset"` removes the effect of that pair, even if
/// it has been set before.
const UNSET: &str = "unset";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Error {
    InvalidLine,
}

struct Document {
    preamble: Preamble,
    sections: Vec<Section>,
}

struct Preamble {
    root: bool,
}

struct Section {
    header: Glob,
    properties: Vec<Property>,
}

struct Glob {}

enum Property {
    IndentStyle(IndentStyle),
    IndentSize(u32),
    TabWidth(u32),
    EndOfLine(EndOfLine),
    Charset(Charset),
    SpellingLanguage(String),
    TrimTrailingWhitespace(bool),
    InsertFinalNewline(bool),

    /// An unrecognized key-value pair.
    Unknown {
        key: String,
        value: String,
    },
}

enum IndentStyle {
    Tab,
    Space,
}

enum EndOfLine {
    Lf,
    Cr,
    Crlf,
}

enum Charset {
    Latin1,
    Utf8,
    Utf8Bom,
    Utf16Be,
    Utf16Le,
}
