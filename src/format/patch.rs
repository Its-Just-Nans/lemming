//! patch format parser
use nom::{
    IResult,
    branch::alt,
    bytes::complete::{tag, take_until, take_while1},
    character::complete::{digit1, newline, space1},
    combinator::opt,
    multi::many_till,
};

use nom::Parser;

#[derive(Debug)]
pub struct FileStat {
    pub path: String,
    pub changed_lines: usize,
}

#[derive(Debug)]
pub struct PatchFile {
    pub commit_hash: String,
    pub author: String,
    pub email: String,
    pub date: String,
    pub subject: String,
    pub file_stats: Vec<FileStat>,
    pub insertions: usize,
    pub deletions: usize,
    pub diffs: Vec<Diff>,
}

#[derive(Debug)]
pub struct Diff {
    pub old_path: String,
    pub new_path: String,
    pub content: String,
}

fn is_hex(c: char) -> bool {
    c.is_ascii_hexdigit()
}

fn parse_commit_hash(input: &str) -> IResult<&str, String> {
    let (input, _) = tag("From ")(input)?;
    let (input, hash) = take_while1(is_hex)(input)?;
    let (input, _) = take_until("\n")(input)?;
    let (input, _) = newline(input)?;
    Ok((input, hash.to_string()))
}

fn parse_author(input: &str) -> IResult<&str, (String, String)> {
    let (input, _) = tag("From: ")(input)?;
    let (input, name) = take_until(" <")(input)?;
    let (input, _) = tag(" <")(input)?;
    let (input, email) = take_until(">")(input)?;
    let (input, _) = tag(">\n")(input)?;
    Ok((input, (name.to_string(), email.to_string())))
}

fn parse_date(input: &str) -> IResult<&str, String> {
    let (input, _) = tag("Date: ")(input)?;
    let (input, date) = take_until("\n")(input)?;
    let (input, _) = newline(input)?;
    Ok((input, date.to_string()))
}

fn parse_subject(input: &str) -> IResult<&str, String> {
    let (input, _) = tag("Subject: ")(input)?;
    let (input, subject) = take_until("\n")(input)?;
    let (input, _) = newline(input)?;
    Ok((input, subject.to_string()))
}

fn parse_file_stat(input: &str) -> IResult<&str, FileStat> {
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
            changed_lines: count.parse().unwrap(),
        },
    ))
}

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

fn parse_summary(input: &str) -> IResult<&str, (usize, usize)> {
    let (input, _) = space1.parse(input)?;
    let (input, files) = digit1.parse(input)?;
    let (input, _) = tag(" files changed, ").parse(input)?;
    let (input, insertions) = digit1.parse(input)?;
    let (input, _) = tag(" insertions").parse(input)?;
    let (input, _) = take_until("\n").parse(input)?;
    let (input, _) = newline.parse(input)?;

    Ok((input, (files.parse().unwrap(), insertions.parse().unwrap())))
}

fn parse_stats(input: &str) -> IResult<&str, (Vec<FileStat>, usize, usize)> {
    let (input, (file_stats, (files_changed, insertions))) =
        many_till(parse_file_stat, parse_summary).parse(input)?;

    Ok((input, (file_stats, files_changed, insertions)))
}

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
            file_stats,
            subject,
            insertions,
            deletions,
            diffs,
        },
    ))
}
