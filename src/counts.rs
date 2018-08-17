use std::io::Write;
use std::io;
use options::Options;

#[derive(Debug)]
pub struct Counts {
    pub word_count: usize,
    pub line_count: usize,
    pub byte_count: usize,
    pub char_count: usize,
    pub is_a_directory: bool,
}

impl Counts {
    pub fn new() -> Self {
        Counts {
            word_count: 0,
            line_count: 0,
            byte_count: 0,
            char_count: 0,
            is_a_directory: false,
        }
    }

    pub fn display<'a>(&self, w: &'a mut Write, filename: &str, opt: &Options) -> io::Result<()> {
        if self.is_a_directory && opt.show_dirs {
            w.write_all(format!("dir {}\n", filename).as_bytes())?;
            w.flush()?;
            return Ok(());
        }
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
