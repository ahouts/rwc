#[macro_use]
extern crate clap;

use clap::{Arg, App};
use std::io::{Read, BufReader, stdin, Error, stdout, stderr, Write};
use std::path::Path;
use std::fs::{File};

#[derive(Debug)]
struct Counts {
    word_count: usize,
    line_count: usize,
    byte_count: usize
}

impl Counts {
    fn new() -> Self {
        Counts{
            word_count: 0,
            line_count: 0,
            byte_count: 0
        }
    }

    fn display(&self, mut w: impl Write, show_bytes: bool, show_lines: bool, show_words: bool) -> Result<(), Error> {
        let mut res = String::new();
        let mut need_tab = false;
        if show_lines {
            res.push_str(self.line_count.to_string().as_str());
            need_tab = true;
        }
        if show_words {
            if need_tab {
                res.push('\t');
            }
            res.push_str(self.word_count.to_string().as_str());
            need_tab = true;
        }
        if show_bytes {
            if need_tab {
                res.push('\t');
            }
            res.push_str(self.byte_count.to_string().as_str());
        }
        res.push('\n');
        w.write_all(res.as_bytes())?;
        w.flush()?;
        Ok(())
    }
}

fn get_reader_for_file(mut filename: &str) -> Result<Box<Read>, Error> {
    filename = filename.trim();
    if filename == "-" {
        return Ok(Box::new(stdin()));
    } else {
        let p = Path::new(filename);
        let f = File::open(p)?;
        return Ok(Box::new(f));
    }
}

fn count_file(filename: &str, counts: &mut Counts, show_lines: bool, show_words: bool) -> Result<(), Error> {
    let reader = get_reader_for_file(filename)?;
    let reader = BufReader::new(reader);
    let mut byte_count: usize = 0;
    let mut bytes = reader.bytes().map(|c| {
        byte_count += 1;
        c
    });
    let mut line_count: usize = 0;
    if show_lines {
        bytes = bytes.map(|c| {
            if c? == b'\n' {
                line_count += 1;
            }
            c
        })
    }
    let mut word_count: usize = 0;
    if show_words {
        let mut in_a_word = false;
        bytes = bytes.map(|c| {
            let is_whitespace = (c?).is_ascii_whitespace();
            if in_a_word && is_whitespace {
                word_count += 1;
            }
            if !in_a_word && !is_whitespace {
                in_a_word = true;
            }
            c
        })
    }
    bytes.for_each(|| {})
    Ok(())
}

fn main() {
    let matches = App::new("rwc")
        .version(crate_version!())
        .author("Andrew Houts <ahouts4@gmail.com>")
        .about("print newline, word, and byte counts for each file")
        .arg(Arg::with_name("bytes")
            .short("c")
            .long("bytes")
            .help("print the byte counts"))
        .arg(Arg::with_name("lines")
            .help("print the newline counts")
            .short("l")
            .long("lines"))
        .arg(Arg::with_name("words")
            .help("print the word counts")
            .short("w")
            .long("words"))
        .arg(Arg::with_name("files")
            .help("FILES to read from.\nWith no FILES, or when a FILE is -, read standard input.")
            .default_value("-")
            .index(1)
            .takes_value(true)
            .multiple(true)
            .long("FILE"))
        .get_matches();
    let mut show_bytes = matches.is_present("bytes");
    let mut show_lines = matches.is_present("lines");
    let mut show_words = matches.is_present("words");

    if !show_words && !show_lines && !show_bytes {
        show_lines = true;
        show_words = true;
        show_bytes = true;
    }

    let files: Vec<&str> = matches.values_of("files").unwrap().collect();
    let mut counts = Counts::new();
    for file in files.into_iter() {
        if let Err(e) = count_file(file, &mut counts, show_lines, show_words) {
            writeln!(stderr(), "{}", e);
            stderr().flush().expect("error writing error to stderr");
            return;
        }
    }
    if let Err(e) = counts.display(stdout(), show_bytes, show_lines, show_words) {
        writeln!(stderr(), "{}", e);
        stderr().flush().expect("error writing error to stderr");
        return;
    }
}
