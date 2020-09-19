use lox_macros::Hello;

trait Hello {
    fn hello(self: &Self);
}

#[derive(Hello)]
enum ByteCode {
    A,
    B(u32),
    C,
}



#[test]
fn get_ast() {
    let a = ByteCode::A;
    a.hello();
    println!("damnn");
}
