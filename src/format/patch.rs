//! patch format parser

use nom::Parser;
use nom::error::Error;
use nom::Err;
use nom::{
    IResult,
    branch::alt,
    bytes::complete::{tag, take_until, take_while1},
    character::complete::{digit1, newline, space1},
    combinator::opt,
    multi::many_till,
};

/// File statistics
#[derive(Debug)]
pub(crate) struct FileStat {
    /// path of file
    pub(crate) path: String,
    /// changed lines
    pub(crate) changed_lines: usize,
}

/// Patch file
#[derive(Debug)]
pub(crate) struct PatchFile {
    /// Commit hash
    pub(crate) commit_hash: String,
    /// Author
    pub(crate) author: String,
    /// Email
    pub(crate) email: String,
    /// Date
    pub(crate) date: String,
    /// Subject
    pub(crate) subject: String,
    /// File stats
    pub(crate) file_stats: Vec<FileStat>,
    /// Number of insertions
    pub(crate) insertions: usize,
    /// Number of deletions
    pub(crate) deletions: usize,
    /// Diffs
    pub(crate) diffs: Vec<Diff>,
}

/// Diff
#[derive(Debug)]
pub(crate) struct Diff {
    /// Old path
    pub(crate) old_path: String,
    /// New path
    pub(crate) new_path: String,
    /// Content
    pub(crate) content: String,
}

/// Check if hex
fn is_hex(c: char) -> bool {
    c.is_ascii_hexdigit()
}

/// Parse commit
fn parse_commit_hash(input: &str) -> IResult<&str, String> {
    let (input, _) = tag("From ")(input)?;
    let (input, hash) = take_while1(is_hex)(input)?;
    let (input, _) = take_until("\n")(input)?;
    let (input, _) = newline(input)?;
    Ok((input, hash.to_string()))
}

/// Parse author
fn parse_author(input: &str) -> IResult<&str, (String, String)> {
    let (input, _) = tag("From: ")(input)?;
    let (input, name) = take_until(" <")(input)?;
    let (input, _) = tag(" <")(input)?;
    let (input, email) = take_until(">")(input)?;
    let (input, _) = tag(">\n")(input)?;
    Ok((input, (name.to_string(), email.to_string())))
}

/// Parse date
fn parse_date(input: &str) -> IResult<&str, String> {
    let (input, _) = tag("Date: ")(input)?;
    let (input, date) = take_until("\n")(input)?;
    let (input, _) = newline(input)?;
    Ok((input, date.to_string()))
}

/// Parse subject
fn parse_subject(input: &str) -> IResult<&str, String> {
    let (input, _) = tag("Subject: ")(input)?;
    let (input, subject) = take_until("\n")(input)?;
    let (input, _) = newline(input)?;
    Ok((input, subject.to_string()))
}

/// Parse file stats
fn parse_file_stats(input: &str) -> IResult<&str, FileStat> {
    let (input, _) = space1.parse(input)?;
    let (input, path) = take_until(" | ").parse(input)?;
    let (input, _) = tag(" | ").parse(input)?;
    let (input, count) = digit1.parse(input)?;
    let (input, _) = space1.parse(input)?;
    let (input, _) = take_while1(|c| c == '+' || c == '-').parse(input)?;
    let (input, _) = newline.parse(input)?;

    Ok((
        input,
        FileStat {
            path: path.to_string(),
            changed_lines: count.parse().map_err(|_e| Err::Failure(Error::new(input, nom::error::ErrorKind::Digit)))?,
        },
    ))
}

/// Parse diff
fn parse_diff(input: &str) -> IResult<&str, Diff> {
    let (input, _) = newline.parse(input)?;
    // diff --git a/foo b/foo
    let (input, _) = tag("diff --git ").parse(input)?;

    let (input, old_path) = tag("a/")
        .and(take_until(" "))
        .map(|(_, p)| p)
        .parse(input)?;

    let (input, _) = space1.parse(input)?;

    let (input, new_path) = tag("b/")
        .and(take_until("\n"))
        .map(|(_, p)| p)
        .parse(input)?;

    let (input, _) = newline.parse(input)?;

    // Everything until next diff, patch end, or EOF
    let (input, content) =
        opt(alt((take_until("\ndiff --git "), take_until("\n--\n")))).parse(input)?;

    let content = content.unwrap_or(input);

    Ok((
        input,
        Diff {
            old_path: old_path.to_string(),
            new_path: new_path.to_string(),
            content: content.to_string(),
        },
    ))
}

/// Parse summary
fn parse_summary(input: &str) -> IResult<&str, (usize, usize)> {
    let (input, _) = space1.parse(input)?;
    let (input, files) = digit1.parse(input)?;
    let (input, _) = tag(" files changed, ").parse(input)?;
    let (input, insertions) = digit1.parse(input)?;
    let (input, _) = tag(" insertions").parse(input)?;
    let (input, _) = take_until("\n").parse(input)?;
    let (input, _) = newline.parse(input)?;

    Ok((
        input,
        (
            files
                .parse()
                .map_err(|_e| Err::Failure(Error::new(input, nom::error::ErrorKind::Digit)))?,
            insertions
                .parse()
                .map_err(|_e| Err::Failure(Error::new(input, nom::error::ErrorKind::Digit)))?,
        ),
    ))
}

/// Parse stats
fn parse_stats(input: &str) -> IResult<&str, (Vec<FileStat>, usize, usize)> {
    let (input, (file_stats, (files_changed, insertions))) =
        many_till(parse_file_stats, parse_summary).parse(input)?;

    Ok((input, (file_stats, files_changed, insertions)))
}

/// Parse patch
pub fn parse_patch(input: &str) -> IResult<&str, PatchFile> {
    let (input, commit_hash) = parse_commit_hash(input)?;
    let (input, (author, email)) = parse_author(input)?;
    let (input, date) = parse_date(input)?;
    let (input, subject) = parse_subject(input)?;

    let (input, _) = take_until("---\n")(input)?;
    let (input, _) = tag("---\n")(input)?;

    let (mut input, (file_stats, insertions, deletions)) = parse_stats(input)?;

    let mut diffs = Vec::new();

    while let Ok((i, diff)) = parse_diff(input) {
        diffs.push(diff);
        input = i;
    }

    Ok((
        input,
        PatchFile {
            commit_hash,
            author,
            email,
            date,
            subject,
            file_stats,
            insertions,
            deletions,
            diffs,
        },
    ))
}
