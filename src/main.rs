use std::fs;
use std::env;
use std::io::{self, prelude::*};

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().collect();
    let filename = &args[1];
    let handle = fs::File::open(filename)?;
    let reader = io::BufReader::new(handle);
    for line in collect_lines(reader)?.iter_mut() {
        split(line);
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

fn split(line: &mut Vec<String>) {
    
}
