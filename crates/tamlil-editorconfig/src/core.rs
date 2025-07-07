/// Characters which converts a line to a comment (thus discardable) when
/// preceding it.
const COMMENT_STARTERS: &[char] = &['#', ';'];

fn parse_file(source: &str) {
    let _ = source
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .filter(|l| !l.starts_with(COMMENT_STARTERS));
}
