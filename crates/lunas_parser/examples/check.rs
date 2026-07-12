//! A tiny `.lunas` checker: parses the file(s) given as arguments and prints
//! any diagnostics in a rustc-like form. Exits non-zero if any error is found.
//!
//! ```sh
//! cargo run -p lunas_parser --example check -- path/to/Component.lunas
//! ```

use lunas_parser::{parse, Severity};
use std::process::ExitCode;

fn main() -> ExitCode {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: check <file.lunas> [more.lunas ...]");
        return ExitCode::from(2);
    }

    let mut had_error = false;
    for path in &paths {
        let src = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{path}: {e}");
                had_error = true;
                continue;
            }
        };
        let (file, diagnostics) = parse(&src);

        if diagnostics.is_empty() {
            println!("{path}: ok");
        } else {
            for d in &diagnostics {
                if d.severity == Severity::Error {
                    had_error = true;
                }
                println!("{path}:");
                println!("{}", d.render(&src, &file.line_index));
            }
        }
    }

    if had_error {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
