use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, Data, DeriveInput, Fields, FieldsUnnamed, Ident, Path, PathSegment, Type,
    TypePath,
};

extern crate proc_macro;

#[proc_macro_derive(ByteCodeEncodeDecode)]
pub fn binary_encode_decode(item: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(item as DeriveInput);
    dbg!(&ast);
    // gen_enum(&ast)
    let a = impl_binary_encode_decode(&ast);
    // dbg!(&a);
    a
    // "".parse().unwrap()
}

fn impl_binary_encode_decode(ast: &DeriveInput) -> TokenStream {
    let ident = &ast.ident;

    if let Data::Enum(data_enum) = &ast.data {
        let variants = &data_enum.variants;

        let enum_variants: Vec<_> = variants
            .iter()
            .map(|variant| {
                let enum_id = &variant.ident;

                let fields = match &variant.fields {
                    Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => unnamed
                        .iter()
                        .map(|field| match &field.ty {
                            Type::Path(TypePath {
                                path: Path { segments, .. },
                                ..
                            }) => &segments[0].ident,
                            _ => panic!("unexpected"),
                        })
                        .collect::<Vec<_>>(),
                    _ => vec![],
                };

                (enum_id, fields)
            })
            .collect();

        let encode = gen_encode(ident, &enum_variants);
        let decode = gen_decode(ident, &enum_variants);

        return (quote! {
            impl ByteCodeEncodeDecode for #ident {
                #encode
                #decode
            }

        })
        .into();
    }

    "".parse().unwrap()
}

fn gen_encode(enum_: &Ident, variants: &Vec<(&Ident, Vec<&Ident>)>) -> proc_macro2::TokenStream {
    let match_arms: Vec<_> = variants
        .iter()
        .enumerate()
        .map(|(i, (ident, fields))| {
            let field_ids: Vec<_> = (0..fields.len()).map(|a| format_ident!("a{}", a)).collect();
            // let tuple_vals = quote! { #(#field_id),* };
            let other_pushes = field_ids.iter().map(
                |tup_field_id| quote! { dest.extend_from_slice(&#tup_field_id.to_ne_bytes()[..]); },
            );
            // let enum_variant = format_ident!("{}::{}", enum_, ident);
            let enum_args = if field_ids.is_empty() {
                quote! {}
            } else {
                quote! {
                (#(#field_ids),*)
                            }
            };
            quote! {
                #enum_::#ident #enum_args => {
                    dest.push(#i as u8);
                    #(#other_pushes)*
                }
            }
        })
        .collect();

    quote! {
        fn encode(&self, dest: &mut Vec<u8>) {
            match self {
                #(#match_arms),*
            };
        }
    }
}

fn gen_decode(enum_: &Ident, variants: &Vec<(&Ident, Vec<&Ident>)>) -> proc_macro2::TokenStream {
    let match_arms: Vec<_> = variants
        .iter()
        .enumerate()
        .map(|(i, (ident, fields))| {
            let field_ids: Vec<_> = (0..fields.len()).map(|a| format_ident!("a{}", a)).collect();
            let field_setters: Vec<_> = field_ids
                .iter()
                .zip(fields)
                .map(|(var, type_)| {
                    // let decode_fn = format_ident!("decode_{}", type_);
                    quote! {
                        let #var = #type_::decode(&mut slice_ptr);
                    }
                })
                .collect();

            // let enum_variant = format_ident!("{}::{}", enum_, ident);

            let enum_args = if field_ids.is_empty() {
                quote! {}
            } else {
                quote! {
                (#(#field_ids),*)
                            }
            };

            quote! {
                #i => {
                    #(#field_setters)*
                    (#enum_::#ident #enum_args, slice_ptr)
                }
            }
        })
        .collect();

    quote! {
        fn decode(src: &[u8]) -> (Self, &[u8]) {
            let mut slice_ptr = &src[1..];
            let byte = src[0];
            match byte as usize {
                #(#match_arms),*,
                _ => {panic!("Invalid instruction byte code")}
            }
        }
    }
}
