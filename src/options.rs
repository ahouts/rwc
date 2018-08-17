
#[derive(Clone)]
pub struct Options {
    pub show_bytes: bool,
    pub show_words: bool,
    pub show_lines: bool,
    pub show_chars: bool,
    pub utf_required: bool,
    pub show_dirs: bool,
}

impl Options {
    pub fn new(show_bytes: bool, show_words: bool, show_lines: bool, show_chars: bool, show_dirs: bool) -> Self {
        if !show_words && !show_lines && !show_bytes && !show_chars {
            Options {
                show_words: true,
                show_bytes: true,
                show_lines: true,
                show_chars: false,
                utf_required: true,
                show_dirs,
            }
        } else {
            Options {
                show_words,
                show_bytes,
                show_lines,
                show_chars,
                utf_required: show_words || show_chars,
                show_dirs,
            }
        }
    }

    pub fn anything_but_bytes(&self) -> bool {
        self.show_lines || self.show_words
    }
}