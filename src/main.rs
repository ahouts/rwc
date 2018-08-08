#[macro_use]
extern crate clap;

use clap::{Arg, App};
use std::io::{Read, stdin, Error, stdout, stderr, Write};
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

    fn display(&self, mut w: impl Write, filename: &str, opt: &Options) -> Result<(), Error> {
        let mut res = String::new();
        if opt.show_lines {
            res.push(' ');
            res.push_str(self.line_count.to_string().as_str());
        }
        if opt.show_words {
            res.push(' ');
            res.push_str(self.word_count.to_string().as_str());
        }
        if opt.show_bytes {
            res.push(' ');
            res.push_str(self.byte_count.to_string().as_str());
        }
        if filename != "-" {
            res.push(' ');
            res.push_str(filename);
        }
        res.push('\n');
        w.write_all(res.as_bytes())?;
        w.flush()?;
        Ok(())
    }
}

fn c_iswspace(c: u8) -> bool {
    c == b' ' ||
        // horizontal tab, line feed, vertical tab, form feed, and carriage return
        (0x09 <= c && c <= 0x0D)
}

fn count_file(filename: &str, counts: &mut Counts, opt: &Options) -> Result<(), Error> {
    let sin = stdin();
    let mut reader: Box<Read> = if filename == "-" {
        Box::new(sin.lock())
    } else {
        let p = Path::new(filename);
        let f = File::open(p)?;
        Box::new(f)
    };
    let mut buff: Vec<u8> = vec![0; 8096];
    let mut in_a_word = false;
    let mut read: usize;
    while {read = reader.read(&mut buff[..])?; read > 0} {
        for byte in &buff[0..read] {
            counts.byte_count += 1;
            if opt.show_lines && *byte == b'\n' {
                counts.line_count += 1;
            }
            if opt.show_words {
                let is_whitespace = c_iswspace(*byte);
                if in_a_word && is_whitespace {
                    counts.word_count += 1;
                }
                if !in_a_word && !is_whitespace {
                    in_a_word = true;
                }
            }
        }
    }
    Ok(())
}

struct Options {
    show_bytes: bool,
    show_words: bool,
    show_lines: bool,
}

impl Options {
    fn new(show_bytes: bool, show_words: bool, show_lines: bool) -> Self {
        Options{
            show_words,
            show_bytes,
            show_lines,
        }
    }

    fn set_default_options(&mut self) {
        self.show_lines = true;
        self.show_words = true;
        self.show_bytes = true;
    }
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
    let mut options = Options::new(matches.is_present("bytes"),
                                   matches.is_present("words"),
                                   matches.is_present("lines"));
    if !options.show_words && !options.show_lines && !options.show_bytes {
        options.set_default_options();
    }

    let files: Vec<&str> = matches.values_of("files").unwrap().collect();
    for file in files.into_iter() {
        let mut counts = Counts::new();
        if let Err(e) = count_file(file, &mut counts, &options) {
            writeln!(stderr(), "{}", e);
            stderr().flush().expect("error writing error to stderr");
            return;
        }
        if let Err(e) = counts.display(stdout(), file, &options) {
            writeln!(stderr(), "{}", e);
            stderr().flush().expect("error writing error to stderr");
            return;
        }
    }
}
