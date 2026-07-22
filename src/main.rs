//! glance binary entry point.
//!
//! Deliberately thin: all logic lives in the library crate (`src/lib.rs`) so it is
//! unit-testable without spawning a process. This file only collects args and forwards
//! the exit code.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(glance::run(&args));
}
