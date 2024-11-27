use core::ops::Range;
use std::{
    cmp::min,
    fs::File,
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
};

use eyre::Result;
use ropey::Rope;


// TODO: Do something about `unwrap`s

// Point.start always points BEFORE the character, Point.end AFTER the character.
pub type Point = Range<usize>;

#[derive(Debug, Clone, Default)]
pub struct Buffer {
    path: Option<PathBuf>,
    pub rope: Rope,
    is_modified: bool,
}

impl Buffer {
    pub fn new() -> Buffer {
        Buffer {
            path: None,
            is_modified: false,
            rope: Rope::new(),
        }
    }
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Buffer> {
        let file = File::open(&path)?;
        let buf: BufReader<File> = BufReader::new(file);
        let rope = Rope::from_reader(buf)?;

        Ok(Buffer {
            path: Some(path.as_ref().to_path_buf()),
            is_modified: false,
            rope,
        })
    }

    pub fn from_string(string: &str) -> Self {
        let rope = Rope::from_str(string);
        Buffer {
            path: None,
            is_modified: false,
            rope,
        }
    }

    pub fn save_as(&self, path: &PathBuf) -> Result<()> {
        let writer = BufWriter::new(File::create(path)?);
        self.rope.write_to(writer)?;
        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        if !self.is_modified || self.path.is_none() {
            return Ok(());
        }
        self.save_as(self.path.as_ref().unwrap())?;
        Ok(())
    }
}

// TODO: Build buffer arena and reference it in the `BufferView`.
// This buffer arena then can be global???

#[derive(Debug, Clone)]
pub struct BufferView {
    // TODO: Think about using SmallVec or something similar. The common case
    // will most likely be one ore very few points (curstors) per view so it
    // makes sense to use something that store values on stack.
    point: Point,
    buffer: Arc<Mutex<Buffer>>,
}

impl BufferView {
    pub fn new(buffer: &Arc<Mutex<Buffer>>) -> BufferView {
        BufferView {
            point: 0..0,
            buffer: buffer.clone(),
        }
    }
    pub fn move_point_forward_char(&mut self) {
        if self.point.end < self.buffer.lock().unwrap().rope.len_chars() {
            self.point.end += 1;
            self.point.start = self.point.end;
        }
    }

    pub fn move_point_backward_char(&mut self) {
        if self.point.start > 0 {
            self.point.start -= 1;
            self.point.end = self.point.start;
        }
    }

    pub fn move_point_end_of_line(&mut self) {
        let line_idx = self.buffer.lock().unwrap().rope.char_to_line(self.point.end);
        let idx = if line_idx == 0 {
            self.buffer.lock().unwrap().rope.len_chars()
        } else {
            self.buffer.lock().unwrap().rope.line_to_char(line_idx + 1) - 1
        };
        self.point.start = idx;
        self.point.end = idx;
    }
    pub fn move_point_start_of_line(&mut self) {
        let line_idx = self.buffer.lock().unwrap().rope.char_to_line(self.point.start);
        self.goto_line(line_idx);
    }
    pub fn move_point_forward_line() {} // TODO: These two have to take into account "visual lines"
                                        // if a line is wrapped and is rendered as two lines, do we move to the next real line or visual line?
    pub fn move_point_backward_line() {}

    pub fn goto_char(&mut self, char_idx: usize) {
        let idx = min(char_idx, self.buffer.lock().unwrap().rope.len_chars());
        self.point.start = idx;
        self.point.end = idx;
    }

    pub fn goto_line(&mut self, line_idx: usize) {
        let buffer = self .buffer .lock().unwrap();
        let idx = buffer
            .rope
            .line_to_char(min(buffer.rope.len_lines(), line_idx));
        self.point.start = idx;
        self.point.end = idx;
    }
    pub fn goto_end_of_buffer(&mut self) {
        let len = { self .buffer .lock().unwrap().rope.len_chars() };
        self.goto_char(len);
    }
    pub fn goto_start_of_buffer(&mut self) {
        self.goto_char(0);
    }

    // Ropey doesn't do searching, but... https://github.com/cessen/ropey/blob/master/examples/search_and_replace.rs
    pub fn search_forward() {}
    pub fn search_forward_rx() {}
    pub fn search_backward() {}
    pub fn search_backward_rx() {}

    // Basic editing.
    pub fn insert_at_point(&mut self, text: &str) {
        let mut buffer = self.buffer.lock().unwrap();
        buffer.rope.insert(self.point.start, text);
        let off = Rope::from(text).len_chars();
        self.point.start += off;
        self.point.end = self.point.start;
        buffer.is_modified = true;
        // TODO: Selection, multiple points, create undo records, ...
    }
    pub fn delete_at_point(&mut self) {
        // Delete, not backspace. For now.
        let p = &self.point;
        let mut buffer = self.buffer.lock().unwrap();
        assert!(p.end <= buffer.rope.len_chars());
        let to = if p.start == p.end {
            min(buffer.rope.len_chars(), p.end + 1)
        } else {
            p.end
        };
        buffer.rope.remove(p.start..to);
        buffer.is_modified = true;
    }
    // TODO: Write this in a way that we can have multiple undo implementations: simple undo/redo stack, undo tree, etc.
    pub fn undo() {}
    pub fn redo() {}

    // Shell integration ;)
    pub fn run_shell_command(&self) -> Result<()> {
        let rope = &self.buffer.lock().unwrap().rope;
        let start = rope.line_to_char(1);
        let end = rope.line_to_char(2);
        let arg = rope.slice(start..end);
        let child = Command::new("echo")
            .arg("-n")
            .arg(arg.to_string())
            .stdout(Stdio::piped())
            .spawn()?;

        let output = child.wait_with_output()?;
        println!("{output:?}");

        // 1. With no selection active, run shell command and capture/show its output.
        // 2. With selection(s) active, pipe the selection(s) to the command and capture/show its output.
        // After the command, have a way to either
        //   a) paste the result into the buffer,
        //   b) replace the buffer? with the output, [probably no; just select the buffer and run the command?]
        //   c) copy the result to system clipboard
        Ok(())
    }

    pub fn buffer<'a>(&'a self) -> std::sync::MutexGuard<'a, Buffer> {
        self.buffer.lock().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::{Buffer, BufferView};

    #[test]
    fn new_buffer() {
        let buf = Arc::new(Mutex::new(Buffer::new()));
        let buf_view = BufferView::new(&buf);
        assert_eq!(buf.lock().unwrap().rope.len_bytes(), 0);
        assert_eq!(buf_view.point.start, 0);
        assert_eq!(buf_view.point.start, buf_view.point.end);
    }

    #[test]
    fn move_in_new_buffer() {
        macro_rules! assert_point {
            ($point:expr) => {
                assert_eq!($point.start, 0);
                assert_eq!($point.start, $point.end);
            };
        }
        let buf = Arc::new(Mutex::new(Buffer::new()));
        let mut buf_view = BufferView::new(&buf);
        buf_view.move_point_backward_char();
        assert_point!(buf_view.point);
        buf_view.move_point_forward_char();
        assert_point!(buf_view.point);
        buf_view.move_point_start_of_line();
        assert_point!(buf_view.point);
        buf_view.move_point_end_of_line();
        assert_point!(buf_view.point);
        buf_view.goto_char(0);
        assert_point!(buf_view.point);
        buf_view.goto_char(10);
        assert_point!(buf_view.point);
        buf_view.goto_line(0);
        assert_point!(buf_view.point);
        buf_view.goto_line(10);
        assert_point!(buf_view.point);
        buf_view.goto_start_of_buffer();
        assert_point!(buf_view.point);
        buf_view.goto_end_of_buffer();
        assert_point!(buf_view.point);
    }
}
