use std::iter::Peekable;
use std::num::{NonZero, NonZeroU32};
use std::ops::RangeInclusive;
use std::str::CharIndices;

use regex::{Match, Regex};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum Error {
    /// Couldn't parse the input as a glob expression.
    Parse,
    /// A range was found that does not hold `num1 < num2` in `num1..num2`.
    InvalidRange,
    NonDirPath,
    RegexCompilation,
}

pub(crate) struct Glob {
    re: Regex,
    num_ranges: Vec<RangeInclusive<i32>>,
}

impl Glob {
    /// # Arguments
    ///
    /// - `ec_dir` - the directory of the EditorConfig file which contains
    ///   `pattern`. Must be an absolute path which doesn't end with a path
    ///   seperator (i.e., `/`), and must have it's path separators normalized
    ///   to `/`.
    pub(crate) fn new<P, S>(ec_dir: P, pattern: S) -> Result<Self, Error>
    where
        P: AsRef<str>,
        S: AsRef<str>,
    {
        if ec_dir.as_ref().ends_with('/') {
            return Err(Error::NonDirPath);
        }

        let pattern = pattern.as_ref();

        let (re, num_ranges) = Parser::new(pattern).parse();

        if !num_ranges.iter().all(|r| r.start() < r.end()) {
            return Err(Error::InvalidRange);
        }

        let has_seperator = pattern.contains('/');
        let starts_with_sep = pattern.starts_with('/');

        let mut regex = "^".to_string() + &regex::escape(ec_dir.as_ref());
        if !has_seperator {
            regex.push_str(".*/");
        } else if !starts_with_sep {
            regex.push('/');
        }
        regex.push_str(&re);
        regex.push('$');

        let re = Regex::new(&regex).map_err(|_| Error::RegexCompilation)?;

        Ok(Self { re, num_ranges })
    }

    #[inline]
    pub(crate) fn is_match<S>(&self, path: S) -> bool
    where
        S: AsRef<str>,
    {
        fn match_in_range(mat: Match, rng: &RangeInclusive<i32>) -> bool {
            mat.as_str().parse().is_ok_and(|n| rng.contains(&n))
        }

        let Some(caps) = self.re.captures(path.as_ref()) else { return false };

        caps.iter()
            .skip(1)
            .map(Option::unwrap)
            .zip(self.num_ranges.iter())
            .all(|(mat, range)| match_in_range(mat, range))
    }
}

struct Parser<'a> {
    pattern: &'a str,
    chars: Peekable<CharIndices<'a>>,
    curr: Option<(usize, char)>,

    are_braces_paired: bool,
    brace_level: Option<NonZeroU32>,
    // inside '[' ... ']'
    is_inside_brackets: bool,

    num_ranges: Vec<RangeInclusive<i32>>,
    regex: String,
}

impl<'a> Parser<'a> {
    fn new(pattern: &'a str) -> Self {
        Self {
            pattern,
            chars: pattern.char_indices().peekable(),
            curr: None,
            are_braces_paired: check_are_braces_paired(pattern),
            brace_level: None,
            is_inside_brackets: false,
            num_ranges: vec![],
            regex: String::with_capacity(pattern.len()),
        }
    }

    fn parse(mut self) -> (String, Vec<RangeInclusive<i32>>) {
        while let Some(ch) = self.bump() {
            match ch {
                '\\' => self.parse_escape(),
                '?' => self.parse_any(),
                '*' => self.parse_star(),
                '[' => self.parse_bracket(),
                '{' => self.parse_open_brace(),
                '}' => self.parse_close_brace(),
                ',' => self.parse_comma(),
                ch => self.parse_literal(ch),
            }
        }

        (self.regex, self.num_ranges)
    }

    fn parse_escape(&mut self) {
        if let Some(ch) = self.bump() {
            if regex_syntax::is_meta_character(ch) {
                self.regex.push('\\');
            } else {
                self.regex.push_str("\\\\");
            }
            self.regex.push(ch);
        } else {
            self.regex.push_str("\\\\");
        }
    }

    fn parse_any(&mut self) {
        self.regex.push_str("[^/]");
    }

    fn parse_star(&mut self) {
        if self.peek().is_some_and(|ch| ch == '*') {
            assert_eq!(self.bump(), Some('*'));
            self.regex.push_str(".*");
        } else {
            self.regex.push_str("[^/]*");
        }
    }

    fn parse_bracket(&mut self) {
        self.regex.push('[');
        if self.peek().is_some_and(|ch| ch == '!') {
            assert_eq!(self.bump(), Some('!'));
            self.regex.push('^');
        }

        let mut escaped = false;

        while let Some(ch) = self.bump() {
            match ch {
                ch if escaped => {
                    escaped = false;
                    self.parse_literal(ch);
                }
                '\\' => escaped = true,
                ']' => {
                    self.regex.push(']');
                    break;
                }
                ch => self.parse_literal(ch),
            }
        }
    }

    fn parse_open_brace(&mut self) {
        if !self.are_braces_paired {
            self.regex.push_str("\\{");
            return;
        }

        let (curr_idx, _ch) = self.curr.unwrap();
        if let Some(closing_brace_offset) =
            is_single_item_braces(&self.pattern[curr_idx..])
        {
            let s = &self.pattern[curr_idx..=curr_idx + closing_brace_offset];

            if let Some(range) = parse_range(s) {
                // HACK: since we can't validate the number being in a range
                // using regular expressions (well, at least not in way that
                // doesn't specify all the possible numbers), we will capture
                // the number, and validate it against the range after matching.
                self.regex.push_str("([\\+\\-]?\\d+)");
                self.num_ranges.push(range);
            } else {
                // If the braces only contains one element, we match it
                // literally (e.g., `{s1}` is `{s1}` and not `s1`).
                s.chars().for_each(|ch| self.parse_literal(ch));
            }

            // Skip 1 because `s` includes the `{` that we bumped to already.
            for _ in (0..s.chars().count()).skip(1) {
                let _ = self.bump().unwrap();
            }
        } else {
            self.regex.push_str("(:?"); // non-capturing group.
            self.increase_brace_level();
        }
    }

    fn parse_close_brace(&mut self) {
        if !self.are_braces_paired {
            self.regex.push_str("\\}");
            return;
        }

        self.regex.push(')');
        self.decrease_brace_level();
    }

    fn parse_comma(&mut self) {
        if self.brace_level.is_some() {
            self.regex.push('|');
        } else {
            self.regex.push_str("\\,");
        }
    }

    fn parse_literal(&mut self, ch: char) {
        if regex_syntax::is_meta_character(ch) {
            self.regex.push('\\');
        }
        self.regex.push(ch);
    }

    fn increase_brace_level(&mut self) {
        const NON_ZERO_ONE: NonZeroU32 = NonZero::new(1).unwrap();

        let l = self.brace_level.map_or(NON_ZERO_ONE, |l| l.saturating_add(1));
        self.brace_level = Some(l);
    }

    fn decrease_brace_level(&mut self) {
        self.brace_level =
            self.brace_level.and_then(|l| NonZero::new(l.get() - 1));
    }

    fn bump(&mut self) -> Option<char> {
        self.curr = self.chars.next();
        self.curr.map(|(_idx, ch)| ch)
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().map(|&(_idx, ch)| ch)
    }
}

fn parse_range(s: &str) -> Option<RangeInclusive<i32>> {
    let (num1, num2) =
        s.strip_prefix('{')?.strip_suffix('}')?.split_once("..")?;
    let start = num1.parse().ok()?;
    let end = num2.parse().ok()?;
    Some(RangeInclusive::new(start, end))
}

fn is_single_item_braces(s: &str) -> Option<usize> {
    let mut escaped = false;

    for (idx, byte) in s.bytes().enumerate() {
        match byte {
            _ if escaped => escaped = false,
            b'\\' => escaped = true,
            b'}' => return Some(idx),
            b',' => return None,
            _ => {}
        }
    }

    None
}

fn check_are_braces_paired(pattern: &str) -> bool {
    let mut escaped = false;
    let mut left = 0;
    let mut right = 0;

    for byte in pattern.bytes() {
        match byte {
            _ if escaped => escaped = false,
            b'\\' => escaped = true,
            b'{' => left += 1,
            b'}' => right += 1,
            _ => {}
        }

        if left < right {
            return false;
        }
    }

    left == right
}
