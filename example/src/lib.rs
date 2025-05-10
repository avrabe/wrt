//! Example WebAssembly Component Model implementation.
//!
//! This crate demonstrates a simple component with a logger resource.

#![warn(clippy::missing_panics_doc)]

use std::cell::{Cell, RefCell};
use wit_bindgen::generate;

generate!({
    inline: r#"
        package my:test;

        interface logging {
            enum level {
                debug,
                info,
                error,
            }

            resource logger {
                constructor(level: level);
                log: func(level: level, msg: string);
                level: func() -> level;
                set-level: func(level: level);
            }
        }

        world my-world {
            export logging;
        }
    "#,
});

use exports::my::test::logging::{Guest, GuestLogger, Level};

struct MyComponent;

// Note that the `logging` interface has no methods of its own but a trait
// is required to be implemented here to specify the type of `Logger`.
impl Guest for MyComponent {
    type Logger = MyLogger;
}

struct MyLogger {
    level: Cell<Level>,
    contents: RefCell<String>,
}

impl GuestLogger for MyLogger {
    fn new(level: Level) -> MyLogger {
        MyLogger { level: Cell::new(level), contents: RefCell::new(String::new()) }
    }

    fn log(&self, level: Level, msg: String) {
        if level as u32 <= self.level.get() as u32 {
            self.contents.borrow_mut().push_str(&msg);
            self.contents.borrow_mut().push('\n');
        }
    }

    fn level(&self) -> Level {
        self.level.get()
    }

    fn set_level(&self, level: Level) {
        self.level.set(level);
    }
}

export!(MyComponent);
