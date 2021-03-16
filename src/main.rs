use std::io::{self, prelude::*};
use std::{cmp, env};
use std::{collections::HashSet, fs};

use regex::{self, Regex};

const ALIGNMENT: usize = 4; // tab size
const COMMENT_PREFIX: Option<char> = Some(';');
const CHANGE_COMMENTS: bool = false;
const STANDALONE_LABEL_COLON: bool = true;

#[derive(PartialEq, Debug)]
enum Size {
    Short,
    Byte,
    Word,
    Long,
    None,
}

// orig_length exists to avoid excessive allocations when composing the processed strings
// (because it lets us create a String::with_capacity(orig_length) instead of a zero-capacity one)
#[derive(Debug)]
enum Line {
    Code {
        orig_length: u16,
        label: Option<String>,
        has_colon: bool,
        initial_ws: u16,
        instruction: String,
        size: Size,
        medial_ws: u16,
        args: Option<String>,
        final_ws: Option<u16>,
        prefix: Option<char>,
        comment: Option<String>,
        collapsible: bool,
    },
    Comment {
        orig_length: u16,
        ws: u16,
        prefix: char,
        text: String,
    },
    Label {
        has_colon: bool,
        orig_length: u16,
        name: String,
        prefix: Option<char>,
        comment: Option<String>,
    },
    Unknown {
        orig_length: u16,
        text: String,
    },
    Blank,
}

struct Regexes<'a> {
    code: &'a Regex,
    label_with_comment: &'a Regex,
    argless_command: &'a Regex,
}

fn main() -> Result<(), io::Error> {
    // annoying alternative to putting these in the actual parse() function and using lazy_static on them
    let code = Regex::new(concat!(
        r"^",
        r"(?P<label>\w+)?(?P<colon>:)?", // optional label
        r"(?P<ws1>\s+)",                 // whitespace before instruction
        r"(?P<instruction>[a-zA-Z]+)(?P<size>\.[SBWL])?", // instruction
        r"(?P<ws2>\s+)",                 // whitespace after instruction
        r"(?P<args>",
        r"(?:#?[$%]?[/a-zA-Z0-9_()\-+]+|#?'[^']+')",
        //   ^prefixes ^reg(list)       ^string
        r"(?:,(?:#?[$%]?[/a-zA-Z0-9_()\-+]+|#?'[^']+'))*",
        r")",
        r"(?P<ws3>\s+)?",                     // whitespace after args
        r"(?P<prefix>[;*])?(?P<comment>.+)?", // comment
        r"$"
    ))
    .unwrap();
    // println!("{}", code.as_str());
    let label_with_comment = Regex::new(
        r"^(?P<label>\w+)?(?P<colon>:)?(?:(?P<ws>\s*)(?P<prefix>[;*])(?P<comment>.+))?$",
    )
    .unwrap();
    let argless_command = Regex::new(
        r"^(?P<label>\w+)?(?P<colon>:)?(?:(?P<ws>\s*)(?P<prefix>[;*])?(?P<comment>.+))?$",
    )
    .unwrap();
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
                    code: &code,
                    label_with_comment: &label_with_comment,
                    argless_command: &argless_command,
                },
            )
        })
        .collect();
    process(&mut parsed);
    for line in transform(&parsed) {
        println!("{}", line);
    }

    Ok(())
}

fn collect_lines<B: BufRead>(reader: B) -> io::Result<Vec<String>> {
    // this function was longer but i trimmed it down & haven't inlined it yet
    reader.lines().collect::<io::Result<Vec<_>>>()
}

fn parse(line: &str, regexes: &Regexes) -> Line {
    // println!("{}", line);
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
            text: trimmed
                .strip_prefix(|s| s == ';' || s == '*')
                .unwrap_or(trimmed)
                .to_owned(),
        };
    }
    if trimmed.starts_with("SIMHALT") || trimmed.starts_with("END") || trimmed.starts_with("RTS") {
        let captures = regexes.argless_command.captures(trimmed).unwrap();
        return Line::Code {
            orig_length: line.len() as u16,
            label: None,
            has_colon: false,
            initial_ws: (post_trimmed.len() - trimmed.len()) as u16,
            instruction: captures.name("label").unwrap().as_str().to_owned(),
            size: Size::None,
            medial_ws: 0,
            args: None,
            final_ws: captures.name("ws").map(|m| m.range().len() as u16),
            prefix: captures
                .name("prefix")
                .and_then(|m| m.as_str().chars().nth(0)),
            comment: captures.name("comment").map(|m| m.as_str().to_owned()),
            collapsible: false,
        };
    }
    // apparently faster/smaller than !trimmed.contains(char::is_ascii_whitespace)
    if !trimmed.as_bytes().iter().any(u8::is_ascii_whitespace) {
        return Line::Label {
            orig_length: post_trimmed.len() as u16,
            has_colon: post_trimmed.ends_with(':'),
            name: post_trimmed.to_owned(),
            prefix: None,
            comment: None,
        };
    }
    if let Some(captures) = regexes.label_with_comment.captures(post_trimmed) {
        return Line::Label {
            has_colon: captures.name("colon").is_some(),
            orig_length: trimmed.len() as u16,
            name: captures.name("label").unwrap().as_str().to_owned(),
            prefix: captures
                .name("prefix")
                .and_then(|m| m.as_str().chars().nth(0)),
            comment: captures.name("comment").map(|m| m.as_str().to_owned()),
        };
    }
    if let Some(captures) = regexes.code.captures(post_trimmed) {
        return Line::Code {
            orig_length: trimmed.len() as u16,
            label: captures.name("label").map(|m| m.as_str().to_owned()),
            has_colon: captures.name("colon").is_some(),
            initial_ws: captures
                .name("ws1")
                .map(|m| m.range().len() as u16)
                .unwrap_or_default(),
            instruction: captures
                .name("instruction")
                .map(|m| m.as_str().to_ascii_uppercase().to_owned())
                .expect("Line of code has no instruction"),
            size: captures
                .name("size")
                .map(|m| match m.as_str() {
                    ".S" => Size::Short,
                    ".B" => Size::Byte,
                    ".W" => Size::Word,
                    ".L" => Size::Long,
                    _ => Size::None,
                })
                .unwrap_or(Size::None),
            medial_ws: captures
                .name("ws2")
                .map(|m| m.range().len() as u16)
                .unwrap_or_default(),
            args: captures.name("args").map(|m| m.as_str().to_owned()),
            final_ws: captures.name("ws3").map(|m| m.range().len() as u16),
            prefix: captures
                .name("prefix")
                .and_then(|m| m.as_str().chars().nth(0)),
            comment: captures.name("comment").map(|m| m.as_str().to_owned()),
            collapsible: false,
        };
    }
    Line::Unknown {
        orig_length: post_trimmed.len() as u16,
        text: post_trimmed.to_owned(),
    }
}

fn process(lines: &mut Vec<Line>) {
    for line in lines.iter_mut() {
        match line {
            Line::Code {
                // orig_length,
                // label,
                // has_colon,
                // initial_ws,
                instruction,
                size,
                // medial_ws,
                args,
                // final_ws,
                prefix,
                // comment,
                collapsible,
                ..
            } => {
                if instruction == "MOVE" && *size == Size::Byte {
                    if let Some(s) = args {
                        // sloppy lol
                        if s.starts_with("#'") && s.contains("',(A5)+") {
                            *collapsible = true;
                        }
                    }
                }
                #[allow(unreachable_code)]
                if CHANGE_COMMENTS {
                    *prefix = COMMENT_PREFIX;
                }
            }
            Line::Comment {
                // orig_length,
                // ws,
                prefix,
                // text,
                ..
            } =>
            {
                #[allow(unreachable_code)]
                if CHANGE_COMMENTS {
                    if let Some(c) = COMMENT_PREFIX {
                        *prefix = c
                    }
                }
            }
            Line::Label {
                // orig_length,
                // name,
                has_colon,
                prefix,
                // comment,
                ..
            } => {
                *has_colon = STANDALONE_LABEL_COLON;
                #[allow(unreachable_code)]
                if CHANGE_COMMENTS {
                    *prefix = COMMENT_PREFIX;
                }
            }
            _ => {}
        };
    }

    // handle sequences of MOVE.B '*'
    let mut movegroup: Vec<usize> = Vec::new();
    for i in 0..lines.len() {
        match &lines[i] {
            Line::Code { collapsible, .. } => {
                if *collapsible {
                    movegroup.push(i);
                } else {
                    match movegroup.len() {
                        2 => handle_movegroup(&movegroup, lines, Size::Word),
                        4 => handle_movegroup(&movegroup, lines, Size::Long),
                        _ => {}
                    }
                    movegroup.clear();
                }
            }
            _ => {}
        }
    }
}

fn handle_movegroup(indices: &Vec<usize>, lines: &mut Vec<Line>, new_size: Size) {
    let mut composed = String::new();
    for i in indices {
        match &lines[*i] {
            Line::Code { args, .. } if args.is_some() => {
                composed.push_str(&args.as_ref().unwrap().chars().nth(2).unwrap().to_string());
            }
            _ => {}
        }
    }
    match &mut lines[indices[0]] {
        Line::Code {
            size,
            args,
            collapsible,
            ..
        } => {
            *size = new_size;
            if let Some(s) = args {
                s.replace_range(3..3, &composed.to_owned());
            }
            *collapsible = false;
        }
        _ => {}
    }
    for i in indices.iter().skip(1).rev() {
        lines.remove(*i);
    }
}

fn transform(lines: &Vec<Line>) -> Vec<String> {
    let mut transformed: Vec<String> = Vec::new();

    // first pass: determine appropriate tabstops
    let mut instruction_tabstop = 0;
    let mut arg_tabstop = 0;
    let mut comment_tabstop = 0;

    let mut original_instruction_tabstops: HashSet<usize> = HashSet::new();

    for i in 0..lines.len() {
        match &lines[i] {
            Line::Code {
                label,
                initial_ws,
                has_colon,
                instruction,
                size,
                args,
                ..
            } => {
                let label_length =
                    label.as_ref().map(|s| s.len()).unwrap_or_default() + *has_colon as usize;
                original_instruction_tabstops.insert(label_length + (*initial_ws as usize));
                instruction_tabstop = cmp::max(instruction_tabstop, label_length);
                arg_tabstop = cmp::max(
                    arg_tabstop,
                    instruction.len()
                        + match size {
                            Size::None => 0,
                            _ => 2,
                        },
                );
                comment_tabstop = cmp::max(
                    comment_tabstop,
                    args.as_ref().map(|s| s.len()).unwrap_or_default(),
                );
            }
            // comments have nothing to contribute to any tabstop, they should just be aligned after this pass
            // and standalone labels should not contribute to the instruction_tabstop i think
            _ => {}
        }
    }

    // snap tabstops to... actual tabstops
    instruction_tabstop = ceiling_div(instruction_tabstop, ALIGNMENT) * ALIGNMENT;
    arg_tabstop = instruction_tabstop + ceiling_div(arg_tabstop, ALIGNMENT) * ALIGNMENT;
    comment_tabstop = arg_tabstop + ceiling_div(comment_tabstop, ALIGNMENT) * ALIGNMENT;

    let original_instruction_tabstop =
        original_instruction_tabstops.iter().sum::<usize>() / original_instruction_tabstops.len();

    // second pass, compose appropriately-aligned strings
    for i in 0..lines.len() {
        println!("{:?}", lines[i]);
        match &lines[i] {
            Line::Code {
                orig_length,
                label,
                has_colon,
                instruction,
                size,
                args,
                prefix,
                comment,
                ..
            } => {
                let mut composed_line = String::with_capacity(*orig_length as usize);
                if let Some(s) = label {
                    composed_line.push_str(s);
                }
                if *has_colon {
                    composed_line.push(':');
                }

                // instead of initial_ws:
                composed_line.push_str(&" ".repeat(instruction_tabstop - composed_line.len()));

                composed_line.push_str(instruction);
                composed_line.push_str(match size {
                    Size::Short => ".S",
                    Size::Byte => ".B",
                    Size::Word => ".W",
                    Size::Long => ".L",
                    Size::None => "",
                });

                if let Some(s) = args {
                    // instead of medial_ws:
                    composed_line.push_str(&" ".repeat(arg_tabstop - composed_line.len()));
                    composed_line.push_str(s);
                }

                if let Some(s) = comment {
                    if let Some(c) = prefix {
                        composed_line.push(*c);
                    }
                    // instead of final_ws:
                    composed_line.push_str(&" ".repeat(comment_tabstop - composed_line.len()));
                    composed_line.push_str(s);
                }

                transformed.push(composed_line);
            }
            Line::Comment {
                orig_length,
                ws,
                prefix,
                text,
            } => {
                // XXX: this logic is very sus lmao, not accounting properly for the diff
                // btwn instruction_tabstop and original_instruction tabstop
                let mut composed_line = String::with_capacity(*orig_length as usize);
                if (*ws as usize) < original_instruction_tabstop / 2 {
                    composed_line.push_str(""); // no-op
                } else if (*ws as usize) <= original_instruction_tabstop {
                    composed_line.push_str(&" ".repeat(instruction_tabstop - composed_line.len()));
                } else if (*ws as usize) < (arg_tabstop - instruction_tabstop) / 2 {
                    composed_line.push_str(&" ".repeat(instruction_tabstop - composed_line.len()));
                } else {
                    composed_line.push_str(&" ".repeat(comment_tabstop - composed_line.len()));
                }
                composed_line.push_str(&prefix.to_string());
                composed_line.push(' '); // spacing after prefix
                composed_line.push_str(text);

                transformed.push(composed_line);
            }
            Line::Label {
                has_colon,
                orig_length,
                name,
                prefix,
                comment,
            } => {
                let mut composed_line = String::with_capacity(*orig_length as usize);
                composed_line.push_str(name);
                if *has_colon {
                    composed_line.push(':');
                }
                if let Some(s) = comment {
                    composed_line.push_str(&" ".repeat(comment_tabstop - composed_line.len()));
                    if let Some(c) = prefix {
                        composed_line.push(*c);
                    }
                    composed_line.push_str(s);
                }

                transformed.push(composed_line);
            }
            Line::Unknown { orig_length, text } => {
                let mut composed_line = String::with_capacity(*orig_length as usize);
                composed_line.push_str(text);

                transformed.push(composed_line);
            }
            Line::Blank => {
                transformed.push(String::new());
            }
        }
    }

    transformed
}

fn ceiling_div(nume: usize, den: usize) -> usize {
    if nume % den == 0 {
        return nume / den;
    }
    return nume / den + 1;
}
