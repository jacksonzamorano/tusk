#[macro_use]
extern crate quote;
extern crate proc_macro;
mod autoquery;
use autoquery::AutoQueryParser;
use proc_macro::TokenStream;
use quote::{format_ident, ToTokens};
use syn::{parse_macro_input, ItemFn, ItemStruct};

#[proc_macro_attribute]
pub fn autoquery(args: TokenStream, input: TokenStream) -> TokenStream {
    let provided_struct = parse_macro_input!(input as ItemStruct);
    let provided_struct_name = &provided_struct.ident;
    let provided_table_name = format!("{}s", provided_struct_name.to_string().to_lowercase());

    let fields = provided_struct
        .fields
        .iter()
        .map(|x| x.ident.as_ref().unwrap().to_string())
        .collect::<Vec<_>>();

    let field_create = fields.iter().map(|x| {
        let x_name_ident = format_ident!("{}", x);
        quote! {
            #x_name_ident: row.get(#x)
        }
    });

    let constructor = quote! {
        pub fn from_postgres(row: &tusk_rs::Row) -> #provided_struct_name {
            #provided_struct_name {
                #(#field_create),*
            }
        }
    };

    let (parsed, params) = AutoQueryParser::parse(args.to_string(), provided_table_name)
        .into_token_stream(provided_struct_name, &fields);

    let creator =
        autoquery::create_insert_fn(provided_struct_name, &provided_struct.fields, &params);

    quote! {
        #provided_struct
        impl #provided_struct_name {
            #constructor
            #creator
            #parsed
        }
    }
    .into()
}

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
            Route::new(
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
            .replace("PostgresConn", "")
            .replace("tusk_rs::", "")
            .replace(['(', ')'], "")
            .trim()
            .to_string()
    );

    let mapped_inputs_outputs = inputs
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

    quote! {
        use core::future::Future;
        use tokio::macros::support::Pin;
        pub fn #data_name() -> Box<fn(Request, tusk_rs::PostgresConn) -> Pin<Box<dyn Future<Output = Result<(#o, Request, tusk_rs::PostgresConn), RouteError>>>>> {
            Box::new(move |a,b| Box::pin(#data_name_int(a,b)))
        }
        async fn #data_name_int(#inputs) -> Result<(#o, Request, tusk_rs::PostgresConn), RouteError> {
            let fn_eval = #data_block;
            return Ok((fn_eval, #(#mapped_inputs_outputs),*));
        }
    }.into()
}

#[proc_macro_derive(ToJson)]
pub fn derive_to_json(item: TokenStream) -> TokenStream {
    let struct_ident = parse_macro_input!(item as ItemStruct);

    let struct_name = struct_ident.ident;
    let struct_fields = struct_ident.fields.iter().map(|x| {
        let x_ident = &x.ident;
        let x_key = x.ident.to_token_stream().to_string();
        quote! {
            output += "\"";
            output += #x_key;
            output += "\" : ";
            output += &self.#x_ident.to_json();
            output += ",";
        }
    });


    let output_new = quote! {
        impl tusk_rs::json::ToJson for #struct_name {
            fn to_json(&self) -> String {
                let mut output = String::new();
                output += "{";
                #(#struct_fields)*
                output = output[0..output.chars().count() - 1].to_string();
                output += "}";
                return output;
            }
        }
    };

    output_new.into()
}

#[proc_macro_derive(FromJson)]
pub fn derive_from_json(item: TokenStream) -> TokenStream {
    let strct = parse_macro_input!(item as ItemStruct);
    let struct_name = &strct.ident;

    let fields_get = strct.fields.iter().map(|x| {
        let x_ident = &x.ident;
        let x_key = x.ident.to_token_stream().to_string();
        quote! {
            #x_ident: json.get(#x_key)?
        }
    });

    let fields_validate_get = strct.fields.iter().map(|x| {
        let x_ident = &x.ident;
        let x_key = x.ident.to_token_stream().to_string();
        let x_msg = format!("{} is required", x_key);
        quote! {
            #x_ident: json.validate_get(#x_key, #x_msg)?
        }
    });

    quote! {
        impl tusk_rs::json::FromJson for #struct_name {
            fn from_json(json: &tusk_rs::json::JsonObject) -> Option<#struct_name> {
                Some(#struct_name {
                    #(#fields_get),*
                })
            }
            fn from_json_validated(json: &tusk_rs::json::JsonObject) -> Result<#struct_name, tusk_rs::RouteError> {
                Ok(#struct_name {
                    #(#fields_validate_get),*
                })
            }
        }
    }.into()
}