use std::env;

use lox::repl::{repl, run_file};

fn main() {
    let args: Vec<_> = env::args().collect();

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
        run_file("./lox/examples/upvalue.lox");
    }
}
