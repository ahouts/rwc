use std::fs::File;
use std::io::Stdin;
use std::io::BufRead;
use std::io::BufReader;

pub enum Reader {
    Stdin(Stdin),
    File(File),
}

impl Reader {
    pub fn get_buff_reader<'a>(&'a mut self) -> Box<BufRead + 'a> {
        match self {
            Reader::Stdin(s) => {
                Box::new(s.lock())
            }
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
