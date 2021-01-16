use std::env;

use lox::repl::run_file;

fn main() {
    let args: Vec<_> = env::args().collect();

    let mut run_repl = false;

    #[cfg(feature = "repl")]
    {
        run_repl = args.len() == 1
    }

    if args.len() == 1 {
        lox::repl::repl()
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
