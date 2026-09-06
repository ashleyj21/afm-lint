mod diagnostic;
mod lint;
mod parser;

use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let path = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("usage: afm-lint <file.afm>");
            return ExitCode::from(2);
        }
    };

    let source = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("error: could not read {}: {}", path, err);
            return ExitCode::from(2);
        }
    };

    let diagnostics = lint::run(&source);
    let lines: Vec<&str> = source.lines().collect();

    for diagnostic in &diagnostics {
        let source_line = lines.get(diagnostic.line - 1).copied().unwrap_or("");
        print!("{}", diagnostic.render(&path, source_line));
        println!();
    }

    if diagnostics.is_empty() {
        println!("no problems found");
        ExitCode::SUCCESS
    } else {
        println!(
            "found {} problem{}",
            diagnostics.len(),
            if diagnostics.len() == 1 { "" } else { "s" }
        );
        ExitCode::FAILURE
    }
}
