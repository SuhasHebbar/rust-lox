use rust_lox::opcodes::{Chunk, Value, Number, Instruction};

fn main() {
    let mut chunk = Chunk::new();

    chunk.add_instruction(Instruction::Return, 123);
    chunk.add_instruction(Instruction::Return, 123);

    let index = chunk.add_value(Value::Number(45.3 as Number));
    chunk.add_instruction(Instruction::Constant(index), 124);


    println!("{}", chunk);
}
