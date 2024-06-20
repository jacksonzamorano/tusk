#[macro_use]
extern crate quote;
extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{format_ident, ToTokens};
use syn::{parse_macro_input, ItemFn, ItemStruct};

/// This macro is used to define a route. It takes in a request type and a route name.
///
/// The arguments are:
/// `#[route(Method Path {: secure_fn (optional)})]`
///
/// Valild methods are:
/// - `Get`
/// - `Post`
/// - `Put`
/// - `Delete`
/// - `Patch`
///
/// If this route is registered using the `Server::module` function,
/// the path provided will be prepended to the path provided in the macro.
/// 
/// This macro should be applied to a function with arguments for:
/// - [`Request`]
/// - [`DatabaseConnection`]
/// - `T`, where `T` is the type provided by treatment function.
///   for more information, see the `tusk_rs::Server` documentation
///
/// It should return a `Result<Response, RouteError>`.
///
/// The route macro all supports security middleware by
/// adding `: your_secure_fn` to the end of the macro.
/// The secure function should be an async function
/// with arguments for:
/// - [`Request`]
/// - [`DatabaseConnection`]
/// - `T`
/// and should return `Result<(Request, DatabaseConnection, T), RouteError>`.
/// Returning an Ok value allows the route to continue, while
/// returning an Err value will return the error to the client.
#[proc_macro_attribute]
pub fn route(args: TokenStream, input: TokenStream) -> TokenStream {
    let params = args
        .to_string()
        .split(' ')
        .map(|x| x.to_string())
        .collect::<Vec<_>>();
    let route_type = params[0].clone();
    let route_type_ident = format_ident!("{}", route_type);

    let idx = params.iter().position(|x| x.trim() == ":");

    let route_name = params[1..if let Some(ix) = idx { ix } else { params.len() }].join("");

    let data = parse_macro_input!(input as ItemFn);
    let data_name = &data.sig.ident;
    let int_fn_name = format_ident!("_int_{}", data_name);

    let data_args = &data.sig.inputs;
    let data_out = &data.sig.output;
    let data_block = &data.block;

    let inputs_last = data
        .sig
        .inputs
        .last()
        .unwrap()
        .to_token_stream()
        .to_string();
    let type_vals = inputs_last.split(':').collect::<Vec<&str>>();
    let mod_type = format_ident!("{}", type_vals[1].to_string().replace(['&', ' '], ""));

    let interceptor = if idx.is_some() {
        let inputs_formatted = data_args
            .iter()
            .map(|x| {
                let name = format_ident!("{}", x.to_token_stream().to_string().split(':').next().unwrap().trim());
                quote! {
                    &#name
                }
            })
            .collect::<Vec<_>>();
        let route_fn = format_ident!("{}", params[idx.unwrap() + 1]);
        quote! {
            #route_fn(#(#inputs_formatted),*).await?;
        }
    } else {
        quote! {}
    };

    quote! {
        pub fn #data_name() -> tusk_rs::Route<#mod_type> {
            tusk_rs::Route::new(
                #route_name.to_string(),
                tusk_rs::RequestType::#route_type_ident,
                Box::new(move |a,b,c| Box::pin(#int_fn_name(a,b,c)))
            )
        }
        async fn #int_fn_name(#data_args) #data_out {
            #interceptor
            #data_block
        }
    }
    .into()
}

/// This macro is used to define a treatment function.
/// The function is almost the same as the [`route`] macro, but it should
/// return a tuple of the response, the request, and the database connection
/// instead of a Response.
#[proc_macro_attribute]
pub fn treatment(_args: TokenStream, input: TokenStream) -> TokenStream {
    let data = parse_macro_input!(input as ItemFn);
    let data_name = &data.sig.ident;
    let data_name_int = format_ident!("_int_{}", data_name);
    let data_block = &data.block;

    let inputs = &data.sig.inputs;

    let o = format_ident!(
        "{}",
        data.sig
            .output
            .to_token_stream()
            .to_string()
            .replace("Result", "")
            .replace(['<', ',', '-'], "")
            .replace("RouteError", "")
            .replace('>', "")
            .replace("Request", "")
            .replace("DatabaseConnection", "")
            .replace("tusk_rs::", "")
            .replace(['(', ')'], "")
            .trim()
            .to_string()
    );

    let mut mapped_inputs_outputs_before = inputs.clone();
    let last_input = mapped_inputs_outputs_before.pop();
    let default_inputs = mapped_inputs_outputs_before.iter().collect::<Vec<_>>();
    let mapped_inputs_outputs = mapped_inputs_outputs_before
        .iter()
        .map(|x| {
            format_ident!(
                "{}",
                x.to_token_stream()
                    .to_string()
                    .split(':')
                    .collect::<Vec<&str>>()[0]
                    .trim()
                    .to_string()
            )
        })
        .collect::<Vec<_>>();

    let param_arg_name = format_ident!("{}", last_input
        .as_ref()
        .unwrap()
        .to_token_stream()
        .to_string()
        .split(':')
        .collect::<Vec<&str>>()[0]
        .trim()
        .to_string());

    let mut is_rc = false;
    let param_arg_type = match last_input.unwrap().value() {
        syn::FnArg::Typed(x) => {
            match x.ty.as_ref() {
                syn::Type::Path(p) => {
                    if p.path.segments.last().to_token_stream().to_string().starts_with("Rc") {
                        match p.path.segments.last().unwrap().arguments {
                            syn::PathArguments::AngleBracketed(ref args) => {
                                is_rc = true;
                                args.args.first().unwrap().to_token_stream()
                            },
                            _ => panic!("Invalid input type"), 
                        }
                    } else {
                        panic!("Unexpected type, either a Rc or a &Type is expected")
                    }
                },
                syn::Type::Reference(rf) => {
                    rf.elem.as_ref().to_token_stream()
                }
                _ =>  panic!("Unexpected type, either a Rc or a &Type is expected")
            }
        },
        _ => panic!("Arg did not match any type"), 
    };

    let param_arg_name_fill = if is_rc { quote! {} } else { quote! { let #param_arg_name = init_param_rced.as_ref(); } };
    let inner_fn_arg_name = if is_rc { quote! { #param_arg_name: std::rc::Rc<#param_arg_type> } } else { quote! { init_param_rced: std::rc::Rc<#param_arg_type> } }; 
    
    quote! {
        use core::future::Future;
        use tokio::macros::support::Pin;
        pub fn #data_name() -> Box<fn(Request, tusk_rs::DatabaseConnection, std::rc::Rc<#param_arg_type>) -> Pin<Box<dyn Future<Output = Result<(#o, Request, tusk_rs::DatabaseConnection), RouteError>>>>> {
            Box::new(move |a,b,c| Box::pin(#data_name_int(a,b,c)))
        }
        async fn #data_name_int(#(#default_inputs),* , #inner_fn_arg_name) -> Result<(#o, Request, tusk_rs::DatabaseConnection), RouteError> {
            #param_arg_name_fill
            let fn_eval = #data_block;
            return Ok((fn_eval, #(#mapped_inputs_outputs),*));
        }
    }.into()
}

#[proc_macro_derive(FromPostgres)]
pub fn derive_from_postgres(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let struct_name = input.ident;
    
    let from_postgres_fields = input.fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let field_name_string = field.ident.as_ref().unwrap().to_string();
        quote! {
            #field_name: row.get(#field_name_string)
        }
    }).collect::<Vec<_>>();
    
    let try_from_postgres_fields = input.fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let field_name_string = field.ident.as_ref().unwrap().to_string();
        quote! {
            #field_name: row.try_get(#field_name_string).map_err(|_| tusk_rs::FromPostgresError::MissingColumn(#field_name_string))?
        }
    }).collect::<Vec<_>>();
    
    quote! {
        impl tusk_rs::FromPostgres for #struct_name {
            fn from_postgres(row: &tusk_rs::Row) -> #struct_name {
                #struct_name {
                    #(#from_postgres_fields),*
                }
            }
            fn try_from_postgres(row: &tusk_rs::Row) -> Result<#struct_name, tusk_rs::FromPostgresError> {
                Ok(#struct_name {
                    #(#try_from_postgres_fields),*
                })
            }
        }
    }.into()
}

#[proc_macro_derive(PostgresJoins)]
pub fn derive_postgres_joins(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let struct_name = input.ident;
    
    quote! {
        impl tusk_rs::PostgresJoins for #struct_name {
            fn joins() -> &'static [&'static tusk_rs::PostgresJoin] {
                &[]
            }
        }
    }.into()
}

#[proc_macro_derive(PostgresReadFields)]
pub fn derive_postgres_read_fields(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let struct_name = input.ident;
    
    let fields = input.fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap().to_string();
        quote! {
            tusk_rs::local!(#field_name)
        }
    }).collect::<Vec<_>>();
    
    quote! {
        impl tusk_rs::PostgresReadFields for #struct_name {
            fn read_fields() -> &'static [&'static tusk_rs::PostgresField] {
                &[#(#fields),*]
            }
        }
    }.into()
}

#[proc_macro_derive(PostgresReadable)]
pub fn derive_postgres_readable(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let struct_name = input.ident;
    
    quote! {
        impl tusk_rs::PostgresReadable for #struct_name {}
    }.into()
}

#[proc_macro_derive(PostgresWriteFields)]
pub fn derive_postgres_write_fields(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let struct_name = input.ident;
    
    let fields = input.fields.iter().map(|field| {
        field.ident.as_ref().unwrap().to_string()
    }).collect::<Vec<_>>();
    
    quote! {
        impl tusk_rs::PostgresWriteFields for #struct_name {
            fn write_fields() -> &'static [&'static str] {
                &[#(#fields),*]
            }
        }
    }.into()
}
#[proc_macro_derive(PostgresWriteable)]
pub fn derive_postgres_writeable(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let struct_name = input.ident;
    
    let fields = input.fields.iter().map(|field| {
        let f = field.ident.as_ref().unwrap();
        let f_name = field.ident.as_ref().unwrap().to_string();
        quote! {
            #f_name => Box::new(std::mem::take(&mut self.#f))
        }
    }).collect::<Vec<_>>();
    
    quote! {
        impl tusk_rs::PostgresWriteable for #struct_name {
            fn write(mut self) -> tusk_rs::PostgresWrite {
                let mut arguments: Vec<Box<(dyn tusk_rs::ToSql + Sync)>> = vec![];
                let fields = <Self as tusk_rs::PostgresWriteFields>::write_fields();
                for f in fields {
                    arguments.push(
                        match *f {
                            #(#fields),*,
                            _ => panic!("Unknown field {}!", f)
                        }
                    )
                }
                tusk_rs::PostgresWrite {
                    fields,
                    arguments
                }
            }
        }
    }.into()
}

/// Embed a file into the binary as a string.
/// This is useful for HTML files or other static files
/// that need to be represented as a string.
/// 
/// The path is derived relative to the project root, which makes
/// it easier to import from /static, /public, or other directories.
#[proc_macro]
pub fn embed(item: TokenStream) -> TokenStream {
    let path = item.to_string().replace('\"', "");
    let resolved_path = std::fs::canonicalize(path).expect("Invalid path!");
    let contents = std::fs::read(&resolved_path).unwrap_or_else(|_| panic!("Could not read contents at {}", resolved_path.display()));
    let contents_string = String::from_utf8(contents).unwrap();
    quote! {
        #contents_string
    }.into()
}

/// Embed a file into the binary as a byte array.
/// This is useful for binary files that need to be represented
/// as a byte array.
///
/// This is similar to [`std::core::include_bytes`], but the path
/// is derived relative to the project root, which makes it easier
/// to import from /static, /public, or other directories.
#[proc_macro]
pub fn embed_binary(item: TokenStream) -> TokenStream {
    let path = item.to_string().replace('\"', "");
    let resolved_path = std::fs::canonicalize(path).expect("Invalid path!");
    let contents = std::fs::read(&resolved_path).unwrap_or_else(|_| panic!("Could not read contents at {}", resolved_path.display()));
    quote! {
        &[#(#contents),*]
    }.into()
}
