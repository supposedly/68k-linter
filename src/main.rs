use std::env;
use std::fs;
use std::io::{self, prelude::*};

use regex::{self, Regex};

const ALIGNMENT: u8 = 4; // tab size

enum Size {
    Short,
    Byte,
    Word,
    Long,
    None,
}

// orig_length exists to avoid excessive allocations when composing the processed strings
// (because it lets us create a String::with_capacity(orig_length) instead of a zero-capacity one)
enum Line {
    Code {
        orig_length: u16,
        label: Option<String>,
        had_colon: bool,
        initial_ws: u16,
        instruction: String,
        size: Size,
        medial_ws: u16,
        args: Option<String>,
        final_ws: Option<u16>,
        comment: Option<String>,
    },
    Comment {
        orig_length: u16,
        ws: u16,
        prefix: char,
        text: String,
    },
    Label {
        had_colon: bool,
        orig_length: u16,
        name: String,
        comment: Option<String>,
    },
    Unknown {
        orig_length: u16,
        text: String,
    },
    Blank,
}

struct Regexes {
    code: Regex,
    label_with_comment: Regex,
}

fn main() -> Result<(), io::Error> {
    // annoying alternative to putting these in the actual parse() function and using lazy_static on them
    let code = Regex::new(concat!(
        r"(?P<label>\w+)?(?P<colon>:)?",                  // optional label
        r"(?P<ws1>\s+)",                                  // whitespace before instruction
        r"(?P<instruction>[a-zA-Z]+)(?P<size>\.[SBWL])?", // instruction
        r"(?P<ws2>\s+)",                                  // whitespace after instruction
        r"(?P<args>",
        r"(?:#?[$%]?[/a-zA-Z0-9]+|#?'[^']+')",
        //            ^prefixes ^reg(list) ^string
        r"(?:,(?:#?[$%]?[/a-zA-Z0-9]+|#?'[^']+'))*",
        r")",
        r"(?P<ws3>\s+)?",    // whitespace after args
        r"(?P<comment>.+)?"  // comment
    ))
    .unwrap();
    // println!("{}", code.as_str());
    let label_with_comment =
        Regex::new(r"(?P<label>\w+)?(?P<colon>:)?(?:\s*[;*](?P<comment>\.+))?").unwrap();
    // println!("{}", label_with_comment.as_str());

    let args: Vec<String> = env::args().collect();
    let filename = &args[1];
    let handle = fs::File::open(filename)?;
    let reader = io::BufReader::new(handle);
    let mut parsed: Vec<Line> = collect_lines(reader)?
        .iter()
        .map(|line| {
            parse(
                line,
                &Regexes {
                    code,
                    label_with_comment,
                },
            )
        })
        .collect();
    process(&mut parsed);

    Ok(())
}

fn collect_lines<B: BufRead>(reader: B) -> io::Result<Vec<String>> {
    // this function was longer but i trimmed it down & haven't inlined it yet
    reader.lines().collect::<io::Result<Vec<_>>>()
}

fn parse(line: &str, regexes: &Regexes) -> Line {
    println!("{}", line);
    let post_trimmed = line.trim_end();
    let trimmed = post_trimmed.trim_start();
    if trimmed.is_empty() {
        return Line::Blank;
    }
    if trimmed.starts_with(|s| s == ';' || s == '*') {
        return Line::Comment {
            orig_length: post_trimmed.len() as u16,
            ws: (post_trimmed.len() - trimmed.len()) as u16,
            prefix: trimmed.chars().nth(0).expect("Line comment went screwy"),
            text: trimmed.to_owned(),
        };
    }
    // apparently faster/smaller than !trimmed.contains(char::is_ascii_whitespace)
    if !trimmed.as_bytes().iter().any(u8::is_ascii_whitespace) {
        return Line::Label {
            orig_length: post_trimmed.len() as u16,
            had_colon: post_trimmed.ends_with(':'),
            name: post_trimmed.to_owned(),
            comment: None,
        };
    }
    match regexes.label_with_comment.captures(post_trimmed) {
        Some(captures) => {
            return Line::Label {
                had_colon: captures.name("colon").is_some(),
                orig_length: trimmed.len() as u16,
                name: captures.name("label").unwrap().as_str().to_owned(),
                comment: captures.name("comment").map(|m| m.as_str().to_owned()),
            }
        }
        None => {}
    };
    match regexes.code.captures(post_trimmed) {
        Some(captures) => Line::Code {
            orig_length: trimmed.len() as u16,
            label: captures.name("label").map(|m| m.as_str().to_owned()),
            had_colon: captures.name("colon").is_some(),
            initial_ws: captures
                .name("ws1")
                .map(|m| m.range().len() as u16)
                .unwrap_or_default(),
            instruction: captures
                .name("instruction")
                .map(|m| m.as_str().to_owned())
                .expect("Line of code has no instruction"),
            size: captures
                .name("size")
                .map(|m| match m.as_str() {
                    "S" => Size::Short,
                    "B" => Size::Byte,
                    "W" => Size::Word,
                    "L" => Size::Long,
                    _ => Size::None,
                })
                .unwrap_or(Size::None),
            medial_ws: captures
                .name("ws2")
                .map(|m| m.range().len() as u16)
                .unwrap_or_default(),
            args: captures.name("args").map(|m| m.as_str().to_owned()),
            final_ws: captures.name("ws3").map(|m| m.range().len() as u16),
            comment: captures.name("comment").map(|m| m.as_str().to_owned()),
        },
        None => Line::Unknown {
            orig_length: post_trimmed.len() as u16,
            text: post_trimmed.to_owned(),
        },
    }
}

fn process(lines: &mut Vec<Line>) {
    for (idx, line) in lines.iter_mut().enumerate() {
        match line {
            Line::Code {
                orig_length,
                label,
                had_colon,
                initial_ws,
                instruction,
                size,
                medial_ws,
                args,
                final_ws,
                comment,
            } => {
                *size = Size::Short;
            }
            Line::Comment {
                orig_length,
                ws,
                prefix,
                text,
            } => {}
            Line::Label { orig_length, name } => {
                name.push_str(":");
            }
            Line::Unknown { orig_length, text } => {}
            Line::Blank => {}
        };
    }
}
