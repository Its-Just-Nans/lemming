//! patch format parser

use bladvak::log;
use nom::{
    Err, IResult, Parser,
    branch::alt,
    bytes::complete::{tag, take_until, take_while, take_while1},
    character::complete::{char, digit1, line_ending, newline, not_line_ending, space0, space1},
    combinator::{map, opt},
    multi::{many_till, many0},
    sequence::{preceded, terminated},
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
    /// Patch metadata
    pub(crate) metadata: Option<PatchMetadata>,
    /// Diffs
    pub(crate) diffs: Vec<Diff>,
}

/// Patch Metadata
#[derive(Debug)]
pub(crate) struct PatchMetadata {
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
    /// More file stats (delete, rename)
    pub(crate) more_file_stats: Vec<String>,
    /// Number of files changes
    pub(crate) files_changes: usize,
    /// Number of insertions
    pub(crate) insertions: usize,
    /// Number of deletions
    pub(crate) deletions: usize,
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
    let (input, subject) = take_until("---")(input)?;
    Ok((input, subject.to_string()))
}

/// Parse file stats
fn parse_file_stats(input: &str) -> IResult<&str, FileStat> {
    let (input, _) = space1.parse(input)?;
    let (input, path) = take_until(" | ").parse(input)?;
    let (input, _) = tag(" | ").parse(input)?;
    let (input, _) = space0.parse(input)?;
    let (input, count) = digit1.parse(input)?;
    // Sometimes the number can be 0 - so no ++ or --
    let (input, _) = space0.parse(input)?;
    let (input, _) = take_while(|c| c == '+' || c == '-').parse(input)?;
    let (input, _) = newline.parse(input)?;
    Ok((
        input,
        FileStat {
            path: path.to_string(),
            changed_lines: count.parse().map_err(|_e| {
                Err::Failure(nom::error::Error::new(input, nom::error::ErrorKind::Digit))
            })?,
        },
    ))
}

/// Parse diff
fn parse_diff(input: &str) -> IResult<&str, Diff> {
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

    let (input, content) = if let Some(some_content) = content {
        (input, some_content)
    } else {
        ("", input)
    };

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
fn parse_summary(input: &str) -> IResult<&str, (usize, usize, usize)> {
    let (input, _) = space1.parse(input)?;
    let (input, files) = digit1.parse(input)?;
    let (input, _) = tag(" files changed").parse(input)?;
    // Optionally parse insertions
    let (input, insertions) = opt(preceded(
        tag(", "),
        terminated(digit1, tag(" insertions(+)")),
    ))
    .parse(input)?;

    // Optionally parse deletions
    let (input, deletions) = opt(preceded(
        tag(", "),
        terminated(digit1, tag(" deletions(-)")),
    ))
    .parse(input)?;
    let (input, _) = take_until("\n").parse(input)?;
    let (input, _) = newline.parse(input)?;

    Ok((
        input,
        (
            files.parse().map_err(|_e| {
                Err::Failure(nom::error::Error::new(input, nom::error::ErrorKind::Digit))
            })?,
            insertions.unwrap_or("0").parse().map_err(|_e| {
                Err::Failure(nom::error::Error::new(input, nom::error::ErrorKind::Digit))
            })?,
            deletions.unwrap_or("0").parse().map_err(|_e| {
                Err::Failure(nom::error::Error::new(input, nom::error::ErrorKind::Digit))
            })?,
        ),
    ))
}

/// Parse stats
fn parse_stats(input: &str) -> IResult<&str, (Vec<FileStat>, usize, usize, usize)> {
    let (input, (file_stats, (files_changes, insertions, deletions))) =
        many_till(parse_file_stats, parse_summary).parse(input)?;

    Ok((input, (file_stats, files_changes, insertions, deletions)))
}

/// Parse many diff
pub(crate) fn parse_many_diffs(input: &str) -> Vec<Diff> {
    let mut diffs = Vec::new();

    let mut input = input;
    loop {
        match parse_diff(input) {
            Ok((input_rest, diff)) => {
                diffs.push(diff);
                let input_rest = if let Ok((input_rest_without_newline, _)) =
                    newline::<&str, nom::error::Error<&str>>.parse(input_rest)
                {
                    input_rest_without_newline
                } else {
                    input_rest
                };
                input = input_rest;
                if input.is_empty() {
                    break;
                }
            }
            Err(e) => {
                log::error!("Error during file parsing: {e}");
                break;
            }
        }
    }
    diffs
}

/// Parse file
pub(crate) fn parse_file(input: &str) -> IResult<&str, PatchFile> {
    if input.starts_with("From") {
        return parse_patch(input);
    }
    let diffs = parse_many_diffs(input);
    Ok((
        input,
        PatchFile {
            metadata: None,
            diffs,
        },
    ))
}

/// Parse patch
pub(crate) fn parse_patch(input: &str) -> IResult<&str, PatchFile> {
    let (input, commit_hash) = parse_commit_hash(input)?;
    let (input, (author, email)) = parse_author(input)?;
    let (input, date) = parse_date(input)?;
    let (input, subject) = parse_subject(input)?;

    let (input, _) = tag("---\n")(input)?;

    let (input, (file_stats, files_changes, insertions, deletions)) = parse_stats(input)?;

    let (input, more) = match newline::<&str, nom::error::Error<&str>>.parse(input) {
        Err(_) => {
            let (input, items) = many0(map(
                terminated(preceded(char(' '), not_line_ending), line_ending),
                |s: &str| s.to_string(),
            ))
            .parse(input)?;
            let (input, _) = newline.parse(input)?;
            (input, items)
        }
        Ok((input, _)) => (input, vec![]),
    };

    let diffs = parse_many_diffs(input);
    Ok((
        input,
        PatchFile {
            metadata: Some(PatchMetadata {
                commit_hash,
                author,
                email,
                date,
                subject,
                file_stats,
                files_changes,
                insertions,
                deletions,
                more_file_stats: more,
            }),
            diffs,
        },
    ))
}
