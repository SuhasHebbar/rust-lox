use std::env;

use lox::repl::run_file;

fn main() {
    let args: Vec<_> = env::args().collect();

    #[cfg(feature = "repl")]
    if args.len() == 1 {
        lox::repl::repl();
        return;
    }

    run_file(&args[1])
}

#[cfg(test)]
mod test {
    use lox::repl::run_file;


    #[test]
    fn main_test() {
        run_file("./lox/examples/upvalue.lox");
    }
}
