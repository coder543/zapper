#![recursion_limit = "128"]

extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use syn::{Data, Fields, Ident, Type};

use proc_macro::TokenStream;

#[proc_macro_derive(ZapEnv, attributes(runner, filter))]
pub fn zap_env_derive(input: TokenStream) -> TokenStream {
    // Parse the string representation
    let ast = syn::parse(input).unwrap();

    // Build the impl
    let gen = impl_zap_env(ast);

    // Return the generated impl
    gen.into()
}

fn impl_zap_env(ast: syn::DeriveInput) -> quote::Tokens {
    let name = ast.ident;
    let mut runner = None;
    let mut filters = vec![];
    for attr in ast.attrs {
        let attr_name = attr.path.segments[0].ident.to_string();
        let val = attr.tts
            .into_iter()
            .skip(1)
            .next()
            .expect(&format!("Error: Expected #[{} = value]", attr_name))
            .to_string()
            .replace("\"", "");
        match attr_name.as_str() {
            "filter" => filters.push(val),
            "runner" => runner = Some(Ident::new(&val, name.span())),
            _ => panic!("unexpected attribute {}", attr_name),
        };
    }

    let runner = runner.expect(&format!(
        "You must provide a #[runner = ZapRunnerStruct] annotation on the \"{}\" struct.",
        name
    ));

    let num_enum = Ident::new(&(runner.to_string() + "Nums"), runner.span());
    let str_enum = Ident::new(&(runner.to_string() + "Strs"), runner.span());
    let filter_enum = Ident::new(&(runner.to_string() + "Filters"), runner.span());

    quote!{
        impl<'a> Environment<'a, #num_enum, #str_enum, #filter_enum> for Provider {
            fn num_constant(&self, name: &str) -> Option<f64> {
                None
            }

            fn str_constant(&self, name: &str) -> Option<&str> {
                None
            }

            fn num_var(name: &str) -> Option<PersonNums> {
                None
            }

            fn str_var(name: &str) -> Option<PersonStrs> {
                None
            }

            fn filter(name: &str) -> Option<(PersonFilters, usize, FilterInput<PersonStrs>)> {
                None
            }
        }
    }
}

#[proc_macro_derive(ZapRunner, attributes(runner, filter))]
pub fn zap_runner_derive(input: TokenStream) -> TokenStream {
    // Parse the string representation
    let ast = syn::parse(input).unwrap();

    // Build the impl
    let gen = impl_zap_runner(ast);

    // panic!("{:#?}", gen);

    // Return the generated impl
    gen.into()
}

fn is_num(ty: Type) -> bool {
    match ty {
        Type::Path(ty_path) => {
            let ty = ty_path.path.segments[0].ident.to_string();
            match ty.as_str() {
                "u8" | "u16" | "u32" | "u64" | "u128" | "i8" | "i16" | "i32" | "i64" | "i128"
                | "f32" | "f64" => true,
                _ => false,
            }
        }
        _ => false,
    }
}

fn impl_zap_runner(ast: syn::DeriveInput) -> quote::Tokens {
    let name = ast.ident;
    let mut filters = vec![];
    for attr in ast.attrs {
        let name = attr.path.segments[0].ident.to_string();
        let val = attr.tts
            .into_iter()
            .skip(1)
            .next()
            .expect(&format!("Error: Expected #[{} = value]", name))
            .to_string()
            .replace("\"", "");
        match name.as_str() {
            "filter" => filters.push(val),
            _ => panic!("unexpected attribute {}", name),
        };
    }
    let mut num_fields = vec![];
    let mut str_fields = vec![];
    match ast.data {
        Data::Struct(data) => match data.fields {
            Fields::Named(fields) => {
                for field in fields.named {
                    let id = field.ident.unwrap();
                    if is_num(field.ty) {
                        num_fields.push(id);
                    } else {
                        str_fields.push(id);
                    }
                }
            }
            _ => panic!("must have named fields"),
        },
        _ => panic!("only works on structs"),
    }
    let num_enum = Ident::new(&(name.to_string() + "Nums"), name.span());
    let str_enum = Ident::new(&(name.to_string() + "Strs"), name.span());
    let filter_enum = Ident::new(&(name.to_string() + "Filters"), name.span());

    let num_match = num_fields
        .iter()
        .map(|f| quote! { #f => self.#f as f64, })
        .collect::<Vec<_>>();
    let str_match = str_fields
        .iter()
        .map(|f| quote! { #f => Cow::from(&*self.#f).into(), })
        .collect::<Vec<_>>();

    quote! {
        use std::borrow::Cow;

        #[derive(Copy, Clone, Debug, PartialEq)]
        enum #num_enum {
            #(#num_fields,)*
        }

        #[derive(Copy, Clone, Debug, PartialEq)]
        enum #str_enum {
            #(#str_fields,)*
        }

        #[derive(Copy, Clone, Debug, PartialEq)]
        enum #filter_enum {}

        impl Runner<#num_enum, #str_enum, #filter_enum> for #name {
            fn num_var(&self, var: #num_enum) -> f64 {
                use #num_enum::*;
                match var {
                   #(#num_match)*
                }
            }

            fn str_var(&self, var: #str_enum) -> Cow<str> {
                use #str_enum::*;
                match var {
                    #(#str_match)*
                }
            }

            fn filter_num(&self, filter: #filter_enum, args: &[f64], input: f64) -> f64 {
                unreachable!()
            }

            fn filter_id(
                &self,
                _filter: #filter_enum,
                _args: &[f64],
                _input_id: #str_enum,
                _buffer: &mut String,
            ) {
                unreachable!()
            }

            fn filter_str(&self, filter: #filter_enum, _args: &[f64], input: Cow<str>, buffer: &mut String) {
                unreachable!()
            }
        }
    }
}
