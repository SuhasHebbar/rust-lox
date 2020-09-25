use lox::{
    opcodes::{Chunk, Instruction, Number, Value},
    vm::Vm,
};

use rustyline::error::ReadlineError;
use rustyline::Config;

const history_save_path: &str = ".lox_history";

fn tear() {
    let mut chunk = Chunk::new();

    let lhs = chunk.add_value(Value::Number(45.3 as Number));
    let rhs = chunk.add_value(Value::Number(21 as Number));
    chunk.add_instruction(Instruction::Constant(lhs), 1);
    chunk.add_instruction(Instruction::Negate, 1);
    chunk.add_instruction(Instruction::Constant(rhs), 1);
    chunk.add_instruction(Instruction::Subtract, 1);
    chunk.add_instruction(Instruction::Return, 1);

    // println!("{}", &chunk);

    let mut vm = Vm::new(chunk);
    vm.run();
}

fn main() {
    let rl_config = Config::builder()
        .history_ignore_dups(true)
        .max_history_size(1000)
        .build();
    let mut rl = rustyline::Editor::<()>::with_config(rl_config);

    if rl.load_history(history_save_path).is_err() {
        println!("Failed to find previous history.");
    }

    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str());
                println!("Printed line: {}", line);
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            _ => {}
        }
    }

    rl.save_history(history_save_path).unwrap();
}
