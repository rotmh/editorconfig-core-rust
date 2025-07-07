macro_rules! case_insensitive_map {
    ($input:expr; $( $string:expr => $value:expr ),* $(,)?) => {
        match $input {
            $( s if s.eq_ignore_ascii_case($string) => Some($value), )*
            _ => None,
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum IndentStyle {
    Tab,
    Space,
}

impl IndentStyle {
    pub(crate) fn parse(s: &str) -> Option<Self> {
        case_insensitive_map! { s;
            "tab" => Self::Tab,
            "space" => Self::Space,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum EndOfLine {
    Lf,
    Cr,
    Crlf,
}

impl EndOfLine {
    pub(crate) fn parse(s: &str) -> Option<Self> {
        case_insensitive_map! { s;
            "lf" => Self::Lf,
            "cr" => Self::Cr,
            "crlf" => Self::Crlf,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Charset {
    Latin1,
    Utf8,
    Utf8Bom,
    Utf16Be,
    Utf16Le,
}

impl Charset {
    pub(crate) fn parse(s: &str) -> Option<Self> {
        case_insensitive_map! { s;
            "latin1" => Self::Latin1,
            "utf-8" => Self::Utf8,
            "utf-8-bom" => Self::Utf8Bom,
            "utf-16be" => Self::Utf16Be,
            "utf-16le" => Self::Utf16Le,
        }
    }
}
