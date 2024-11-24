#![windows_subsystem = "windows"]
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
