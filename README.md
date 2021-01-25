# rust-lox
`rust-lox` is a rust implementation of the Lox Language Bytecode interpreter covered in the [Crafting Interpreters](https://craftinginterpreters.com/) book.

To build the project use

```
# Use the --release flag to get a release build with more optimisations.
cargo build [--release]
```

To run a lox script use
```
cargo run [--release] [filepath]
```