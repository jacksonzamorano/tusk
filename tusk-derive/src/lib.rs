#[macro_use]
extern crate quote;
extern crate proc_macro;
use proc_macro::TokenStream;
use quote::{format_ident, ToTokens};
use syn::{parse_macro_input, ItemFn, ItemStruct};

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
        pub fn #data_name() -> Box<fn(Request, tusk_rs::DatabaseConnection) -> Pin<Box<dyn Future<Output = Result<(#o, Request, tusk_rs::DatabaseConnection), RouteError>>>>> {
            Box::new(move |a,b| Box::pin(#data_name_int(a,b)))
        }
        async fn #data_name_int(#inputs) -> Result<(#o, Request, tusk_rs::DatabaseConnection), RouteError> {
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
    let generics = struct_ident.generics;
    let impl_types = generics.params.iter().map(|x| {
        let d = format_ident!("{}", x.to_token_stream().to_string().split(':').next().unwrap().trim());
        quote! {#d}
    }).collect::<Vec<_>>();
    let impl_insert = if impl_types.is_empty() { quote!{} } else {quote!{<#(#impl_types),*>}};

    let output_new = quote! {
        impl #generics tusk_rs::json::ToJson for #struct_name #impl_insert {
            fn to_json(&self) -> String {
                let mut output = String::new();
                output += "{";
                #(#struct_fields)*
                output.pop();
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

#[proc_macro_derive(PostgresReadFields)]
pub fn derive_postgres_read_fields(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let struct_name = input.ident;
    
    let fields = input.fields.iter().map(|field| {
        field.ident.as_ref().unwrap().to_string()
    }).collect::<Vec<_>>().join(",");
    
    quote! {
        impl tusk_rs::PostgresReadFields for #struct_name {
            fn read_fields() -> &'static str {
                #fields 
            }
        }
    }.into()
}

#[proc_macro_derive(PostgresReadable)]
pub fn derive_postgres_readable(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let struct_name = input.ident;
    
    quote! {
        impl tusk_rs::PostgresReadable for #struct_name {
            fn required_joins() -> &'static str {
                ""
            }
        }
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