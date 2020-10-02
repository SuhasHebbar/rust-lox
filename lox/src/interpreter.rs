pub enum InterpreterResult {
    Ok,
    CompileError,
    RuntimeError,
}

pub struct Interpreter {}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {}
    }

    pub fn interpret(&mut self, source: &str) -> InterpreterResult {
        self.compile(source);

        InterpreterResult::Ok
    }

    fn compile(&mut self, source: &str) {
        todo!()
    }
}
