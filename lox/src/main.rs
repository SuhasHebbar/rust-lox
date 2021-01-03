use std::env;

use lox::repl::{repl, run_file};

// fn main2() {
//     let mut chunk = Chunk::new();

//     let lhs = chunk.add_value(Value::Number(45.3 as Number));
//     let rhs = chunk.add_value(Value::Number(21 as Number));
//     chunk.add_instruction(Instruction::Constant(lhs), 1);
//     chunk.add_instruction(Instruction::Negate, 1);
//     chunk.add_instruction(Instruction::Constant(rhs), 1);
//     chunk.add_instruction(Instruction::Subtract, 1);
//     chunk.add_instruction(Instruction::Return, 1);

//     println!("{}", &chunk);

//     let mut vm = Vm::new(chunk);
//     vm.run();
// }

fn main() {
    let args: Vec<_> = env::args().collect();

    // runFile("main.lox");

    if args.len() == 1 {
        repl()
    } else {
        run_file(&args[1])
    }
}

#[cfg(test)]
mod test {
    use lox::repl::run_file;


    #[test]
    fn main_test() {
        run_file("test.lox");
    }
}
