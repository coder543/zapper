#![recursion_limit = "128"]

extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use syn::{Data, Fields, Ident, Type};

use proc_macro::TokenStream;

#[proc_macro_derive(ZapperEnv, attributes(runner))]
pub fn zapper_env_derive(input: TokenStream) -> TokenStream {
    // Parse the string representation
    let ast = syn::parse(input).unwrap();

    // Build the impl
    let gen = impl_zapper_env(ast);

    // Return the generated impl
    gen.into()
}

fn impl_zapper_env(ast: syn::DeriveInput) -> quote::Tokens {
    let name = ast.ident;
    let mut runner = None;
    let mut filters = vec![];
    for attr in ast.attrs {
        let attr_name = attr.path.segments[0].ident.to_string();
        let val = attr
            .tts
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

    let runner = runner.expect(&format!(
        "You must provide a #[runner = ZapperRunnerStruct] annotation on the \"{}\" struct.",
        name
    ));

    let num_enum = Ident::new(&(runner.to_string() + "Nums"), runner.span());
    let str_enum = Ident::new(&(runner.to_string() + "Strs"), runner.span());
    let filter_enum = Ident::new(&(runner.to_string() + "Filters"), runner.span());

    let num_match = num_fields
        .iter()
        .map(|f| {
            let fs = f.to_string();
            quote! { #fs => Some(self.#f as f64), }
        })
        .collect::<Vec<_>>();

    let str_match = str_fields
        .iter()
        .map(|f| {
            let fs = f.to_string();
            quote! { #fs => ::std::borrow::Cow::from(&*self.#f).into(), }
        })
        .collect::<Vec<_>>();

    quote!{
        #[allow(bad_style, unused)]
        impl<'a> ::zapper::Environment<'a, #num_enum, #str_enum, #filter_enum> for Provider {
            fn num_constant(&self, name: &str) -> Option<f64> {
                match name {
                    #(#num_match)*
                    _ => None
                }
            }

            fn str_constant(&self, name: &str) -> Option<::std::borrow::Cow<str>> {
                match name {
                    #(#str_match)*
                    _ => None
                }
            }

            fn num_var(name: &str) -> Option<#num_enum> {
                #num_enum::from_str(name)
            }

            fn str_var(name: &str) -> Option<#str_enum> {
                #str_enum::from_str(name)
            }

            fn filter(name: &str) -> Option<(#filter_enum, usize, ::zapper::FilterInput<#str_enum>)> {
                #filter_enum::from_str(name)
            }
        }
    }
}

#[proc_macro_derive(ZapperRunner, attributes(filter))]
pub fn zapper_runner_derive(input: TokenStream) -> TokenStream {
    // Parse the string representation
    let ast = syn::parse(input).unwrap();

    // Build the impl
    let gen = impl_zapper_runner(ast);

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

fn impl_zapper_runner(ast: syn::DeriveInput) -> quote::Tokens {
    let name = ast.ident;
    let mut filters = vec![];
    for attr in ast.attrs {
        let name = attr.path.segments[0].ident.to_string();
        let val = attr
            .tts
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
    let filter_fields = filters.iter().map(|f| {
        Ident::new(
            &f[..f
                   .find('/')
                   .expect("filters must specify number of args and return type.")],
            name.span(),
        )
    });

    let num_match = num_fields
        .iter()
        .map(|f| quote! { #num_enum::#f => self.#f as f64, })
        .collect::<Vec<_>>();

    let num_from = num_fields
        .iter()
        .map(|f| {
            let fs = f.to_string();
            quote! { #fs => Some(#num_enum::#f), }
        })
        .collect::<Vec<_>>();

    let str_match = str_fields
        .iter()
        .map(|f| quote! { #str_enum::#f => ::std::borrow::Cow::from(&*self.#f).into(), })
        .collect::<Vec<_>>();

    let str_from = str_fields
        .iter()
        .map(|f| {
            let fs = f.to_string();
            quote! { #fs => Some(#str_enum::#f), }
        })
        .collect::<Vec<_>>();

    let mut num_filters = vec![];
    let mut str_filters = vec![];
    let mut custom_filters = vec![];

    let filter_from = filters
        .iter()
        .map(|f| {
            let split = f
                .find('/')
                .expect("filters must specify number of args and return type.");
            let filter = &f[..split];
            let filter_i = Ident::new(&filter, name.span());
            let arg_count = f[split + 1..f.len() - 1]
                .parse::<usize>()
                .expect("argument count for filter must be a usize");
            let filter_type = f.as_bytes()[f.len() - 1] as char;
            match filter_type {
            'n' => {
                num_filters.push(quote! { #filter_enum::#filter_i => #filter_i(self, args, input), });
                quote!( #filter => Some((#filter_enum::#filter_i, #arg_count, ::zapper::FilterInput::Numeric)), )
            }
            's' => {
                str_filters.push(quote! { #filter_enum::#filter_i => #filter_i(self, args, &input, buffer), });                
                quote!( #filter => Some((#filter_enum::#filter_i, #arg_count, ::zapper::FilterInput::Stringified)), )
            }
            'x' => {
                custom_filters.push(quote! { #filter_enum::#filter_i => #filter_i(self, args, input_id, buffer), });                                
                quote!( #filter => Some((#filter_enum::#filter_i, #arg_count, ::zapper::FilterInput::StrEnumId(vec![]))), )
            }
            _ => panic!("no such input type as {}, valid options are n (numeric), s (stringified), x (custom)", filter_type)
        }
        })
        .collect::<Vec<_>>();

    // println!(
    //     "{:#?}",
    quote! {
        #[allow(bad_style)]
        #[derive(Copy, Clone, Debug, PartialEq)]
        enum #num_enum {
            #(#num_fields,)*
        }

        impl #num_enum {
            fn from_str(name: &str) -> Option<#num_enum> {
                match name {
                    #(#num_from)*
                    _ => None
                }
            }
        }

        #[allow(bad_style)]
        #[derive(Copy, Clone, Debug, PartialEq)]
        enum #str_enum {
            #(#str_fields,)*
        }

        impl #str_enum {
            fn from_str(name: &str) -> Option<#str_enum> {
                match name {
                    #(#str_from)*
                    _ => None
                }
            }
        }

        #[allow(bad_style)]
        #[derive(Copy, Clone, Debug, PartialEq)]
        enum #filter_enum {
            #(#filter_fields,)*
        }

        impl #filter_enum {
            fn from_str(name: &str) -> Option<(#filter_enum, usize, ::zapper::FilterInput<#str_enum>)> {
                match name {
                    #(#filter_from)*
                    _ => None
                }
            }
        }

        #[allow(bad_style, unused)]
        impl ::zapper::Runner<#num_enum, #str_enum, #filter_enum> for #name {
            fn num_var(&self, var: #num_enum) -> f64 {
                match var {
                   #(#num_match)*
                }
            }

            fn str_var(&self, var: #str_enum) -> ::std::borrow::Cow<str> {
                match var {
                    #(#str_match)*
                }
            }

            fn filter_num(&self, filter: #filter_enum, args: &[f64], input: f64) -> f64 {
                match filter {
                    #(#num_filters)*
                    _ => unreachable!("bug in zapper! attempted to execute {:?} as a numeric filter erroneously", filter)
                }
            }

            fn filter_str(&self, filter: #filter_enum, args: &[f64], input: ::std::borrow::Cow<str>, buffer: &mut String) {
                match filter {
                    #(#str_filters)*
                    _ => unreachable!("bug in zapper! attempted to execute {:?} as a string filter erroneously", filter)
                }
            }

            fn filter_id(
                &self,
                filter: #filter_enum,
                args: &[f64],
                input_id: #str_enum,
                buffer: &mut String,
            ) {
                match filter {
                    #(#custom_filters)*
                    _ => unreachable!("bug in zapper! attempted to execute {:?} as a custom filter erroneously", filter)
                }
            }
        }
    }
    // );
    // unreachable!();
}
