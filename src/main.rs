use rust_lox::opcodes::{Chunk, OpCode, Value, Number};

fn main() {
    let mut chunk = Chunk::new();

    chunk.add_instruction(OpCode::Return, 123);
    chunk.add_instruction(OpCode::Return, 123);

    let index = chunk.add_value(Value::Number(45.3 as Number));
    chunk.add_instruction(OpCode::Constant, 124);
    chunk.add_byte(index, 124);


    println!("{}", chunk.disassemble("test chunk"));
}
