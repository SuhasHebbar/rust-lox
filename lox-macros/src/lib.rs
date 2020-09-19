use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput};

extern crate proc_macro;

#[proc_macro_derive(ToBytes)]
pub fn bytecode(item: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(item as DeriveInput);
    dbg!(&ast);
    gen_enum(&ast)
}

fn gen_enum(ast: &DeriveInput) -> TokenStream {
    let ident = &ast.ident;
    // if let Data::Enum(data_enum) = &ast.data {
    //     let variants = &data_enum.variants;

    // let enum_variants: Vec<_> = variants.iter().map(|variant| {
    //     let enum_variant = &variant.ident;
    //     quote! {
    //         #enum_variant,
    //     }
    // }).collect();

    // let struct_def = quote! {
    //     #[derive(Debug, PartialEq)]
    //     pub enum ByteCode {
    //         #(#enum_variants)*
    //     }
    // };

    //     return struct_def.into();
    // }

    let struct_def = quote! {
        impl Hello for #ident {
            fn hello(&self) {
                println!("Hello!");
            }
        }
    };

    return struct_def.into();

    "".parse().unwrap()
}
