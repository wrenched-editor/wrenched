#![windows_subsystem = "windows"]
mod buffer {
    use core::ops::Range;
    use std::{
        cmp::min,
        fmt, fs,
        fs::File,
        io,
        io::BufReader,
        path::PathBuf,
        process::{Command, Stdio},
    };

    use ropey::Rope;

    // Point.start always points BEFORE the character, Point.end AFTER the character.
    type Point = Range<usize>;

    pub struct Buffer {
        path: Option<PathBuf>,
        point: Point,
        rope: Rope,
        is_modified: bool,
    }

    impl Buffer {
        pub fn new() -> Buffer {
            Buffer {
                path: None,
                is_modified: false,
                point: Point::from(0..0),
                rope: Rope::new(),
            }
        }

        pub fn load(path: PathBuf) -> Buffer {
            let file = File::open(path.clone()).expect("CANT OPEN");
            let buf: BufReader<File> = BufReader::new(file);
            let rope = Rope::from_reader(buf).expect("CANT CREATE ROPEY");

            Buffer {
                path: Some(path),
                is_modified: false,
                point: Point::from(0..0),
                rope,
            }
        }

        // TODO: Define custom error/result?
        pub fn save_as(&self, path: &PathBuf) -> io::Result<()> {
            fs::write(path, self.rope.to_string())
        }

        pub fn save(&self) -> io::Result<()> {
            if !self.is_modified || self.path.is_none() {
                return Ok(());
            }
            self.save_as(self.path.as_ref().unwrap())
        }

        pub fn move_point_forward_char(&mut self) {
            if self.point.end < self.rope.len_chars() {
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
            let line_idx = self.rope.char_to_line(self.point.end);
            let idx = if line_idx == 0 {
                self.rope.len_chars()
            } else {
                self.rope.line_to_char(line_idx + 1) - 1
            };
            self.point.start = idx;
            self.point.end = idx;
        }
        pub fn move_point_start_of_line(&mut self) {
            let line_idx = self.rope.char_to_line(self.point.start);
            self.goto_line(line_idx);
        }
        pub fn move_point_forward_line() {} // TODO: These two have to take into account "visual lines"
                                            // if a line is wrapped and is rendered as two lines, do we move to the next real line or visual line?
        pub fn move_point_backward_line() {}

        pub fn goto_char(&mut self, char_idx: usize) {
            let idx = min(char_idx, self.rope.len_chars());
            self.point.start = idx;
            self.point.end = idx;
        }

        pub fn goto_line(&mut self, line_idx: usize) {
            let idx = self.rope.line_to_char(min(self.rope.len_lines(), line_idx));
            self.point.start = idx;
            self.point.end = idx;
        }
        pub fn goto_end_of_buffer(&mut self) {
            self.goto_char(self.rope.len_chars());
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
            self.rope.insert(self.point.start, text);
            let off = Rope::from(text).len_chars();
            self.point.start += off;
            self.point.end = self.point.start;
            self.is_modified = true;
            // TODO: Selection, multiple points, create undo records, ...
        }
        pub fn delete_at_point(&mut self) {
            // Delete, not backspace. For now.
            let p = &self.point;
            assert!(p.end <= self.rope.len_chars());
            let to = if p.start == p.end {
                min(self.rope.len_chars(), p.end + 1)
            } else {
                p.end
            };
            self.rope.remove(p.start..to);
            self.is_modified = true;
        }
        // TODO: Write this in a way that we can have multiple undo implementations: simple undo/redo stack, undo tree, etc.
        pub fn undo() {}
        pub fn redo() {}

        // Shell integration ;)
        pub fn run_shell_command(&self) {
            let start = self.rope.line_to_char(1);
            let end = self.rope.line_to_char(2);
            let arg = self.rope.slice(start..end);
            let child = Command::new("echo")
                .arg("-n")
                .arg(arg.to_string())
                .stdout(Stdio::piped())
                .spawn()
                .expect("Failed to start echo process");

            match child.wait_with_output() {
                Ok(out) => println!("{out:?}"),
                Err(err) => panic!("{err}"),
            }

            // 1. With no selection active, run shell command and capture/show its output.
            // 2. With selection(s) active, pipe the selection(s) to the command and capture/show its output.
            // After the command, have a way to either
            //   a) paste the result into the buffer,
            //   b) replace the buffer? with the output, [probably no; just select the buffer and run the command?]
            //   c) copy the result to system clipboard
        }
    }

    impl fmt::Display for Buffer {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let idx = self.point.end;
            let left = self.rope.slice(..idx);
            let right = self.rope.slice(idx..);
            write!(f, "{}X{}", left, right);
            Ok(())
        }
    }
    mod tests {
        use super::Buffer;
        #[test]
        fn new_buffer() {
            let buf = Buffer::new();
            assert_eq!(buf.rope.len_bytes(), 0);
            assert_eq!(buf.point.start, 0);
            assert_eq!(buf.point.start, buf.point.end);
        }

        #[test]
        fn move_in_new_buffer() {
            macro_rules! assert_point {
                ($point:expr) => {
                    assert_eq!($point.start, 0);
                    assert_eq!($point.start, $point.end);
                };
            }
            let mut buf = Buffer::new();
            buf.move_point_backward_char();
            assert_point!(buf.point);
            buf.move_point_forward_char();
            assert_point!(buf.point);
            buf.move_point_start_of_line();
            assert_point!(buf.point);
            buf.move_point_end_of_line();
            assert_point!(buf.point);
            buf.goto_char(0);
            assert_point!(buf.point);
            buf.goto_char(10);
            assert_point!(buf.point);
            buf.goto_line(0);
            assert_point!(buf.point);
            buf.goto_line(10);
            assert_point!(buf.point);
            buf.goto_start_of_buffer();
            assert_point!(buf.point);
            buf.goto_end_of_buffer();
            assert_point!(buf.point);
        }
    }
}

// Distinguish between editor commands and keybindings.
// Keymap: key/object map, where the object is either an editor command, or a nested keymap.
// Extension language: lua?
// Config: TOML?
// Q&A:
//   1) Does ropey normalize line endings to LF? If not, do we care?
// Wishlist (in no particular order):
//   jsynacek:
//     - [edit] delete-trailing-whitespace
//     - [lang] first-class markdown support
//     - first-class git support

struct Command {}
// new file
// save file, save as file
// open file
// revert file
// quit editor

struct Editor {}

use std::path::PathBuf;

//fn main() {
//    let buffer = Buffer::load(PathBuf::from("./test.txt"));
//
//    //buffer.insert_at_point("HELLOS3\nfoo\nbar\newlines");
//    // println!("TEXT: {}", buffer);
//    // match buffer.save() {
//    //     Ok(_) => println!("OK: Saved."),
//    //     Err(e) => println!("ERR: Not saved: {e}"),
//    // }
//    buffer.run_shell_command();
//}

// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

// On Windows platform, don't show a console when opening the app.
use winit::error::EventLoopError;
use xilem::{
    view::{button, checkbox, flex, textbox, Axis, FlexSpacer},
    EventLoop, EventLoopBuilder, WidgetView, Xilem,
};

use crate::buffer::Buffer;

struct Task {
    description: String,
    done: bool,
}

struct TaskList {
    next_task: String,
    tasks: Vec<Task>,
}

impl TaskList {
    fn add_task(&mut self) {
        if self.next_task.is_empty() {
            return;
        }
        self.tasks.push(Task {
            description: std::mem::take(&mut self.next_task),
            done: false,
        });
    }
}

fn app_logic(task_list: &mut TaskList) -> impl WidgetView<TaskList> {
    let input_box = textbox(
        task_list.next_task.clone(),
        |task_list: &mut TaskList, new_value| {
            task_list.next_task = new_value;
        },
    )
    .on_enter(|task_list: &mut TaskList, _| {
        task_list.add_task();
    });
    let first_line = flex((
        input_box,
        button("Add task".to_string(), |task_list: &mut TaskList| {
            task_list.add_task();
        }),
    ))
    .direction(Axis::Vertical);

    let tasks = task_list
        .tasks
        .iter()
        .enumerate()
        .map(|(i, task)| {
            let checkbox = checkbox(
                task.description.clone(),
                task.done,
                move |data: &mut TaskList, checked| {
                    data.tasks[i].done = checked;
                },
            );
            let delete_button = button("Delete", move |data: &mut TaskList| {
                data.tasks.remove(i);
            });
            flex((checkbox, delete_button)).direction(Axis::Horizontal)
        })
        .collect::<Vec<_>>();

    flex((
        FlexSpacer::Fixed(40.), // HACK: Spacer for Androird
        first_line,
        tasks,
    ))
}

fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let data = TaskList {
        // Add a placeholder task for Android, whilst the
        next_task: "My Next Task".into(),
        tasks: vec![
            Task {
                description: "Buy milk".into(),
                done: false,
            },
            Task {
                description: "Buy eggs".into(),
                done: true,
            },
            Task {
                description: "Buy bread".into(),
                done: false,
            },
        ],
    };

    let app = Xilem::new(data, app_logic);
    app.run_windowed(event_loop, "First Example".into())
}

// Boilerplate code for android: Identical across all applications

#[cfg(not(target_os = "android"))]
#[allow(dead_code)]
// This is treated as dead code by the Android version of the example, but is actually live
// This hackery is required because Cargo doesn't care to support this use case, of one
// example which works across Android and desktop
fn main() -> Result<(), EventLoopError> {
    run(EventLoop::with_user_event())
}
