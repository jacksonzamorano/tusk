#[macro_use]
extern crate quote;
extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{format_ident, ToTokens};
use syn::{
    parse_macro_input, Data, DeriveInput, ItemEnum,
    ItemStruct,
};

#[proc_macro_derive(FromPostgres)]
pub fn derive_from_postgres(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let struct_name = input.ident;

    let from_postgres_fields = input
        .fields
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let field_name_string = field.ident.as_ref().unwrap().to_string();
            quote! {
                #field_name: row.get(#field_name_string)
            }
        })
        .collect::<Vec<_>>();

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
    }
    .into()
}

#[proc_macro_derive(PostgresReadFields)]
pub fn derive_postgres_read_fields(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let struct_name = input.ident;

    let fields = input
        .fields
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap().to_string();
            quote! {
                tusk_rs::local!(#field_name)
            }
        })
        .collect::<Vec<_>>();

    quote! {
        impl tusk_rs::PostgresReadFields for #struct_name {
            fn read_fields() -> &'static [&'static tusk_rs::PostgresField] {
                &[#(#fields),*]
            }
        }
    }
    .into()
}

#[proc_macro_derive(PostgresReadable)]
pub fn derive_postgres_readable(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let struct_name = input.ident;

    quote! {
        impl tusk_rs::PostgresReadable for #struct_name {}
    }
    .into()
}

#[proc_macro_derive(PostgresWriteFields)]
pub fn derive_postgres_write_fields(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let struct_name = input.ident;

    let fields = input
        .fields
        .iter()
        .map(|field| field.ident.as_ref().unwrap().to_string())
        .collect::<Vec<_>>();

    quote! {
        impl tusk_rs::PostgresWriteFields for #struct_name {
            fn write_fields() -> &'static [&'static str] {
                &[#(#fields),*]
            }
        }
    }
    .into()
}
#[proc_macro_derive(PostgresWriteable)]
pub fn derive_postgres_writeable(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let struct_name = input.ident;

    let fields = input
        .fields
        .iter()
        .map(|field| {
            let f = field.ident.as_ref().unwrap();
            let f_name = field.ident.as_ref().unwrap().to_string();
            quote! {
                #f_name => Box::new(std::mem::take(&mut self.#f))
            }
        })
        .collect::<Vec<_>>();

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
    }
    .into()
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
    let contents = std::fs::read(&resolved_path)
        .unwrap_or_else(|_| panic!("Could not read contents at {}", resolved_path.display()));
    let contents_string = String::from_utf8(contents).unwrap();
    quote! {
        #contents_string
    }
    .into()
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
    let contents = std::fs::read(&resolved_path)
        .unwrap_or_else(|_| panic!("Could not read contents at {}", resolved_path.display()));
    quote! {
        &[#(#contents),*]
    }
    .into()
}

/// Derive a ToJson implementation.
#[proc_macro_derive(ToJson)]
pub fn derive_to_json(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    match input.data {
        Data::Struct(struct_ident) => {
            let struct_name = input.ident;
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
            let generics = input.generics;
            let impl_types = generics
                .params
                .iter()
                .map(|x| {
                    let d = format_ident!(
                        "{}",
                        x.to_token_stream()
                            .to_string()
                            .split(':')
                            .next()
                            .unwrap()
                            .trim()
                    );
                    quote! {#d}
                })
                .collect::<Vec<_>>();
            let impl_insert = if impl_types.is_empty() {
                quote! {}
            } else {
                quote! {<#(#impl_types),*>}
            };

            let output_new = quote! {
                impl #generics tusk_rs::ToJson for #struct_name #impl_insert {
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
        Data::Enum(enum_ident) => {
            let name = &input.ident;
            let opts = enum_ident
                .variants
                .iter()
                .map(|x| {
                    let ident = &x.ident;
                    let ident_str = format!("\"{}\"", x.ident.to_string());
                    quote! {
                        Self::#ident => #ident_str.to_string()
                    }
                })
                .collect::<Vec<_>>();
            quote! {
                impl tusk_rs::ToJson for #name {
                    fn to_json(&self) -> String {
                        match self {
                            #(#opts),*
                        }
                    }
                }
            }
            .into()
        }
        _ => panic!("Cannot derive for this kind!"),
    }
}

#[proc_macro_derive(JsonRetrieve)]
pub fn derive_json_retrieve(item: TokenStream) -> TokenStream {
    let enm = parse_macro_input!(item as ItemEnum);
    let struct_name = &enm.ident;
    let struct_name_str = enm.ident.to_string();

    let fields_map = enm
        .variants
        .iter()
        .map(|x| {
            let name = &x.ident;
            let str = x.ident.to_token_stream().to_string();
            quote! {
                #str => Ok(Self::#name)
            }
        })
        .collect::<Vec<_>>();

    quote! {
        impl tusk_rs::JsonRetrieve for #struct_name {
            fn parse(key: String, value: Option<&String>) -> Result<Self, tusk_rs::JsonParseError> {
                let value = value.ok_or_else(|| tusk_rs::JsonParseError::NotFound(key.clone()))?;
                match value.as_str() {
                    #(#fields_map),*,
                    _ => return Err(tusk_rs::JsonParseError::InvalidType(key, #struct_name_str))
                }
            }
        }
    }
    .into()
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

    quote! {
        impl tusk_rs::FromJson for #struct_name {
            fn from_json(json: &tusk_rs::JsonObject) -> Result<#struct_name, tusk_rs::JsonParseError> {
                Ok(#struct_name {
                    #(#fields_get),*
                })
            }
        }
    }.into()
}
