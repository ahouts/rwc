#[macro_use]
extern crate clap;
extern crate utf8;
extern crate rayon;
extern crate glob;

use clap::{App, ArgMatches};
use utf8::BufReadDecoder;
use rayon::spawn;
use glob::glob;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::io;
use std::io::{BufRead, stdin, stdout, stderr, Write};
use std::path::Path;
use std::fs::{File, metadata};

mod counts;
mod options;
mod reader;

use counts::Counts;
use options::Options;
use reader::Reader;

fn make_app<'a, 'b>() -> App<'a, 'b> {
    clap_app!(rwc =>
        (author: "Andrew Houts <ahouts4@gmail.com>")
        (about: "print newline, word, and byte counts for each file.")
        (version: crate_version!())
        (long_about:
            "print newline, word, and byte counts for each file.\n\nWhen no flags \
            are set; lines, chars, and bytes will be selected by default. The results will \
            be displayed in the following order:\n\n<line count> <word count> <byte count> <char count> \
            <file>"
        )
        (@arg bytes: -c --bytes "print the byte counts")
        (@arg chars: -m --chars "print the character counts")
        (@arg lines: -l --lines "print the newline counts")
        (@arg words: -w --words "print the word counts")
        (@arg dirs: -d --dirs "show directories in output")
        (@arg files:
            --FILE
            +takes_value
            +multiple
            +required
            index(1)
            default_value("-")
            long_help("FILES to read from. When a file is \"-\", read standard input. Supports \
            bash style globbing (eg. **/*.js for all javascript files in current directory \
            recursively). Surround globs in quotes to ensure your shell doesn't try to expand \
            the glob.")
            help("FILES to read from. When a file is \"-\", read standard input. Supports \
            bash style globbing.")
        )
    )
}

fn get_options(matches: &ArgMatches) -> Options {
    Options::new(
        matches.is_present("bytes"),
        matches.is_present("words"),
        matches.is_present("lines"),
        matches.is_present("chars"),
        matches.is_present("dirs"),
    )
}

fn read_as_utf8(mut r: Reader, counts: &mut Counts, cfg: &Options) -> io::Result<()> {
    let mut utf_reader = BufReadDecoder::new(r.get_buff_reader());
    let mut in_a_word = false;
    counts.byte_count = 0;
    while let Some(s) = utf_reader.next_lossy() {
        let s: &str = s?;
        for c in s.chars() {
            counts.char_count += 1;
            counts.byte_count += c.len_utf8();
            if cfg.show_lines && c == '\n' {
                counts.line_count += 1;
            }
            if cfg.show_words {
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
    counts.byte_count = 0;
    loop {
        let bytes_read;
        {
            let buf = reader.fill_buf()?;
            bytes_read = buf.len();
            counts.byte_count += bytes_read;
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
        reader.consume(bytes_read);
    }
    Ok(())
}

fn count_file(filename: &str, counts: &mut Counts, opt: &Options) -> io::Result<()> {
    let reader = if filename == "-" {
        Reader::from(stdin())
    } else {
        let p = Path::new(filename);
        if p.is_file() {
            let f = File::open(p)?;
            Reader::from(f)
        } else {
            counts.is_a_directory = true;
            return Ok(());
        }
    };
    if opt.show_bytes && filename != "-" {
        counts.byte_count = metadata(filename)?.len() as usize;
    }
    if opt.only_bytes() && filename != "-" {
        return Ok(());
    }
    if opt.utf_required {
        return read_as_utf8(reader, counts, opt);
    } else {
        return read_as_bytes(reader, counts, opt);
    }
}

fn spawn_result_displayer(result_receiver: Receiver<io::Result<(String, Counts)>>, done_sender: Sender<()>, options: &Options) {
    spawn({
        let options = options.clone();
        move || {
            result_receiver
                .into_iter()
                .for_each(|res| {
                    let (filename, counts) = match res {
                        Ok(r) => r,
                        Err(e) => {
                            writeln!(stderr(), "{}", e).expect("error writing error to stderr");
                            stderr().flush().expect("error writing error to stderr");
                            return;
                        }
                    };
                    let sout = stdout();
                    let mut sout_lock = sout.lock();
                    if let Err(e) = counts.display(&mut sout_lock, filename.as_str(), &options) {
                        writeln!(stderr(), "{}", e).expect("error writing error to stderr");
                        stderr().flush().expect("error writing error to stderr");
                        std::process::exit(-1);
                    }
                });
            done_sender.send(()).expect("failed to send done status");
        }
    });
}

fn spawn_glob_processor(file_globs: Vec<&str>, filename_sender: Sender<String>) {
    file_globs
        .into_iter()
        .for_each(|f| {
            if f == "-" {
                filename_sender.send(String::from(f)).expect("failed to send filename");
                return;
            }
            let g = match glob(f) {
                Err(e) => {
                    writeln!(stderr(), "{}", e).expect("error writing error to stderr");
                    stderr().flush().expect("error writing error to stderr");
                    return;
                }
                Ok(g) => g,
            };
            let filename_sender = filename_sender.clone();
            spawn(move || {
                g.for_each(|entry| {
                    match entry {
                        Ok(path) => {
                            filename_sender.send(String::from(path.to_str().expect("error reading path"))).expect("failed to send filename");
                        }
                        Err(e) => {
                            writeln!(stderr(), "{}", e).expect("error writing error to stderr");
                            stderr().flush().expect("error writing error to stderr");
                        }
                    }
                });
            });
        });
}

fn spawn_file_processor(filename_receiver: Receiver<String>, result_sender: Sender<io::Result<(String, Counts)>>, options: &Options) {
    filename_receiver
        .into_iter()
        .for_each(|file| {
            let options = options.clone();
            let result_sender = result_sender.clone();
            spawn(move || {
                let mut counts = Counts::new();
                if let Err(e) = count_file(file.as_ref(), &mut counts, &options) {
                    result_sender.send(Err(e)).expect("error sending result over channel");
                } else {
                    result_sender.send(Ok((file, counts))).expect("error sending result over channel");
                }
            });
        });
}

fn main() {
    let app = make_app();
    let matches = app.get_matches();
    let options = get_options(&matches);

    let file_globs: Vec<&str> = matches
        .values_of("files")
        .expect("error reading files")
        .collect();
    let (result_sender, result_receiver) = channel::<io::Result<(String, Counts)>>();
    let (done_sender, done_receiver) = channel::<()>();
    spawn_result_displayer(result_receiver, done_sender, &options);

    let (filename_sender, filename_receiver) = channel::<String>();
    spawn_glob_processor(file_globs, filename_sender);

    spawn_file_processor(filename_receiver, result_sender, &options);

    done_receiver.recv().expect("failed to receive done status");
}
