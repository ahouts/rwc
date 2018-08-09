#[macro_use]
extern crate clap;
extern crate utf8;
extern crate rayon;

use clap::{Arg, App};
use utf8::BufReadDecoder;
use rayon::prelude::*;
use std::io;
use std::io::{BufRead, BufReader, stdin, stdout, stderr, Write, Stdin};
use std::path::Path;
use std::fs::File;

#[derive(Debug)]
struct Counts {
    word_count: usize,
    line_count: usize,
    byte_count: usize,
    char_count: usize,
}

impl Counts {
    fn new() -> Self {
        Counts{
            word_count: 0,
            line_count: 0,
            byte_count: 0,
            char_count: 0,
        }
    }

    fn display<'a>(&self, w: &'a mut Write, filename: &str, opt: &Options) -> io::Result<()> {
        let mut res = String::new();
        let mut space_needed = false;
        if opt.show_lines {
            res.push_str(self.line_count.to_string().as_str());
            space_needed = true;
        }
        if opt.show_words {
            if space_needed {
                res.push(' ');
            }
            res.push_str(self.word_count.to_string().as_str());
            space_needed = true;
        }
        if opt.show_bytes {
            if space_needed {
                res.push(' ');
            }
            res.push_str(self.byte_count.to_string().as_str());
            space_needed = true;
        }
        if opt.show_chars {
            if space_needed {
                res.push(' ');
            }
            res.push_str(self.char_count.to_string().as_str());
            space_needed = true;
        }
        if filename != "-" {
            if space_needed {
                res.push(' ');
            }
            res.push_str(filename);
        }
        res.push('\n');
        w.write_all(res.as_bytes())?;
        w.flush()?;
        Ok(())
    }
}

enum Reader{
    Stdin(Stdin),
    File(File),
}

impl Reader{
    fn get_buff_reader<'a>(&'a mut self) -> Box<BufRead + 'a> {
        match self {
            Reader::Stdin(s) => {
                Box::new(s.lock())
            },
            Reader::File(f) => {
                Box::new(BufReader::new(f))
            }
        }
    }
}

impl From<File> for Reader {
    fn from(f: File) -> Self {
        Reader::File(f)
    }
}

impl From<Stdin> for Reader {
    fn from(s: Stdin) -> Self {
        Reader::Stdin(s)
    }
}

fn read_as_utf8(mut r: Reader, counts: &mut Counts, opt: &Options) -> io::Result<()> {
    let mut utf_reader = BufReadDecoder::new(r.get_buff_reader());
    let mut in_a_word = false;
    while let Some(s) = utf_reader.next_lossy() {
        let s: &str = s?;
        for c in s.chars() {
            if opt.show_bytes {
                counts.byte_count += c.len_utf8();
            }
            counts.char_count += 1;
            if opt.show_lines && c == '\n' {
                counts.line_count += 1;
            }
            if opt.show_words {
                let is_whitespace = c.is_ascii_whitespace();
                if in_a_word && is_whitespace {
                    counts.word_count += 1;
                }
                in_a_word = !is_whitespace;
            }
        }
    }
    Ok(())
}

fn read_as_bytes(mut r: Reader, counts: &mut Counts, cfg: &Options) -> io::Result<()> {
    let mut reader = r.get_buff_reader();
    loop {
        let bytes_read;
        {
            let buf = reader.fill_buf()?;
            bytes_read = buf.len();
            if bytes_read == 0 {
                break;
            }
            if cfg.show_lines {
                for byte in buf {
                    if *byte == b'\n' {
                        counts.line_count += 1;
                    }
                }
            }
        }
        counts.byte_count += bytes_read;
        reader.consume(bytes_read);
    }
    Ok(())
}

fn count_file(filename: &str, counts: &mut Counts, opt: &Options) -> io::Result<()> {
    let reader= if filename == "-" {
        Reader::from(stdin())
    } else {
        let p = Path::new(filename);
        let f = File::open(p)?;
        Reader::from(f)
    };
    if opt.utf_required {
        return read_as_utf8(reader, counts, opt);
    } else {
        return read_as_bytes(reader, counts, opt);
    }
}

#[derive(Clone)]
struct Options {
    show_bytes: bool,
    show_words: bool,
    show_lines: bool,
    show_chars: bool,
    utf_required: bool,
}

impl Options {
    fn new(show_bytes: bool, show_words: bool, show_lines: bool, show_chars: bool) -> Self {
        if !show_words && !show_lines && !show_bytes && !show_chars {
            Options{
                show_words: true,
                show_bytes: true,
                show_lines: true,
                show_chars: false,
                utf_required: true,
            }
        } else {
            Options {
                show_words,
                show_bytes,
                show_lines,
                show_chars,
                utf_required: show_words || show_chars,
            }
        }
    }
}

fn main() {
    let matches = App::new("rwc")
        .version(crate_version!())
        .author("Andrew Houts <ahouts4@gmail.com>")
        .about("print newline, word, and byte counts for each file.")
        .long_about("print newline, word, and byte counts for each file.\n\nWhen no flags \
        are set; lines, chars, and bytes will be selected by default. The results will \
        be displayed in the following order:\n\n<line count> <word count> <byte count> <char count> \
        <file>")
        .arg(Arg::with_name("bytes")
            .short("c")
            .long("bytes")
            .help("print the byte counts"))
        .arg(Arg::with_name("chars")
            .short("m")
            .long("chars")
            .help("print the character counts"))
        .arg(Arg::with_name("lines")
            .help("print the newline counts")
            .short("l")
            .long("lines"))
        .arg(Arg::with_name("words")
            .help("print the word counts")
            .short("w")
            .long("words"))
        .arg(Arg::with_name("files")
            .help("FILES to read from. When a file is \"-\", read standard input.")
            .default_value("-")
            .index(1)
            .takes_value(true)
            .multiple(true)
            .long("FILE"))
        .get_matches();
    let options = Options::new(matches.is_present("bytes"),
                                   matches.is_present("words"),
                                   matches.is_present("lines"),
                                   matches.is_present("chars"));

    let files: Vec<String> = matches.values_of("files")
        .unwrap()
        .map(|f| String::from(f))
        .collect();

    files.into_par_iter().map(|file: String| {
        let options = options.clone();
        let mut counts = Counts::new();
        count_file(file.as_ref(), &mut counts, &options)?;
        Ok((file, counts))
    }).for_each(|res: io::Result<(String, Counts)>| {
        let (filename, counts) = match res {
            Ok(r) => r,
            Err(e) => {
                writeln!(stderr(), "{}", e).expect("error writing error to stderr");
                stderr().flush().expect("error writing error to stderr");
                return;
            },
        };
        let sout = stdout();
        let mut sout_lock = sout.lock();
        if let Err(e) = counts.display(&mut sout_lock, filename.as_str(), &options) {
            writeln!(stderr(), "{}", e).expect("error writing error to stderr");
            stderr().flush().expect("error writing error to stderr");
            return;
        }
    });
}
