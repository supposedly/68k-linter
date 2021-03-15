use std::fs;
use std::env;
use std::io::{self, prelude::*};

enum Size {
    Short,
    Byte,
    Word,
    Long
}

enum Line {
    Code {
      label: Option<String>,
      initial_ws: u16,
      instruction: String,
      size: Size,
      medial_ws: u16,
      args: Vec<String>,
      final_ws: Option<u16>,
      comment: Option<String>,
    },
    Comment {
      ws: u16,
      text: String
    },
    Label {
      name: String
    }
}

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().collect();
    let filename = &args[1];
    let handle = fs::File::open(filename)?;
    let reader = io::BufReader::new(handle);
    for line in collect_lines(reader)?.iter_mut() {
        parse(line);
    }
    Ok(())
}

fn collect_lines<B: BufRead>(reader: B) -> io::Result<Vec<Vec<String>>> {
    let lines = reader.lines()
      .collect::<Result<Vec<_>, _>>()?;

    Ok(lines.into_iter()
      .map(|s| vec![s])
      .collect())
}

fn parse(line: &mut Vec<String>) -> Vec<Line> {
    todo!();
}
