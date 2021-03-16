use std::env;
use std::fs;
use std::io::{self, prelude::*};

use regex::{self, Regex};

enum Size {
    Short,
    Byte,
    Word,
    Long,
    None,
}

enum Line {
    Code {
        label: Option<String>,
        initial_ws: u16,
        instruction: String,
        size: Size,
        medial_ws: u16,
        args: Option<String>,
        final_ws: Option<u16>,
        comment: Option<String>,
    },
    Comment {
        ws: u16,
        prefix: char,
        text: String,
    },
    Label {
        name: String,
    },
    Unknown {
        text: String,
    },
    Blank,
}

fn main() -> Result<(), io::Error> {
    // annoying alternative to putting it in the actual parse() function and using lazy_static on it
    let code: Regex = Regex::new(concat!(
        r"(?P<label>\w+)?:?",                             // optional label
        r"(?P<ws1>\s+)",                                  // whitespace before instruction
        r"(?P<instruction>[a-zA-Z]+)(?P<size>\.[SBWL])?", // instruction
        r"(?P<ws2>\s+)",                                  // whitespace after instruction
        r"(?P<args>",
        r"(?:#?[$%]?[/a-zA-Z0-9]+|#'[^']+')",
        //            ^prefixes ^reg(list) ^string
        r"(?:,(?:#?[$%]?[/a-zA-Z0-9]+|#'[^']+'))*",
        r")",
        r"(?P<ws3>\s+)?",    // whitespace after args
        r"(?P<comment>.+)?"  // comment
    ))
    .unwrap();
    println!("{}", code.as_str());

    let args: Vec<String> = env::args().collect();
    let filename = &args[1];
    let handle = fs::File::open(filename)?;
    let reader = io::BufReader::new(handle);
    for line in collect_lines(reader)?.iter() {
        parse(line, &code);
    }

    Ok(())
}

fn collect_lines<B: BufRead>(reader: B) -> io::Result<Vec<String>> {
    reader.lines().collect::<io::Result<Vec<_>>>()
}

fn parse(line: &str, code_regex: &Regex) -> Line {
    println!("{}", line);
    let post_trimmed = line.trim_end();
    let trimmed = post_trimmed.trim_start();
    if trimmed.is_empty() {
        return Line::Blank;
    }
    if trimmed.starts_with(|s| s == ';' || s == '*') {
        return Line::Comment {
            ws: (post_trimmed.len() - trimmed.len()) as u16,
            prefix: trimmed.chars().nth(0).expect("Line comment went screwy"),
            text: trimmed.to_owned(),
        };
    }
    // apparently faster/smaller than !trimmed.contains(char::is_ascii_whitespace)
    if !trimmed.as_bytes().iter().any(u8::is_ascii_whitespace) {
        return Line::Label {
            name: post_trimmed.to_owned(),
        };
    }
    match code_regex.captures(post_trimmed) {
        Some(captures) => Line::Code {
            label: captures.name("label").map(|m| m.as_str().to_owned()),
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
                    _ => Size::None
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
            text: line.to_owned(),
        },
    }
}
