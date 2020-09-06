use rust_lox::{vm::Vm, opcodes::{Chunk, Value, Number, Instruction}};

fn main() {
    let mut chunk = Chunk::new();

    let index = chunk.add_value(Value::Number(45.3 as Number));
    chunk.add_instruction(Instruction::Constant(index), 1);
    chunk.add_instruction(Instruction::Negate, 1);
    chunk.add_instruction(Instruction::Return, 1);

    // println!("{}", &chunk);

    let mut vm = Vm::new(chunk);
    vm.run();
}
