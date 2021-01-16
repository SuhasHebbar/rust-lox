use crate::interpreter::{Interpreter, InterpreterResult};
use std::fs::File;
use std::io::prelude::*;
use std::process;

pub fn run_file(file_path: &str) {
    let mut file = File::open(file_path).expect("Failed to open file");

    let mut content = String::new();
    file.read_to_string(&mut content);

    let mut interpreter = Interpreter::new();

    let result = interpreter.interpret(&content);

    match result {
        InterpreterResult::CompileError => process::exit(65),
        InterpreterResult::RuntimeError => process::exit(70),
        _ => {
            // do nothing for now
        }
    };
}

const HISTORY_SAVE_PATH: &str = ".lox_history";

pub fn repl() {
    #[cfg(feature = "repl")]
    {
        use rustyline::{error::ReadlineError, Config};
        let rl_config = Config::builder()
            .history_ignore_dups(true)
            .max_history_size(1000)
            .build();
        let mut rl = rustyline::Editor::<()>::with_config(rl_config);

        if rl.load_history(HISTORY_SAVE_PATH).is_err() {
            eprintln!("Failed to find previous history.");
        }

        let mut interpreter = Interpreter::new();

        loop {
            let readline = rl.readline(">> ");
            match readline {
                Ok(line) => {
                    rl.add_history_entry(line.as_str());
                    println!("Printed line: {}", line);
                    interpreter.interpret(&line);
                }
                Err(ReadlineError::Interrupted) => {
                    println!("CTRL-C");
                    break;
                }
                _ => {}
            }
        }

        rl.save_history(HISTORY_SAVE_PATH).unwrap();
    }
}
