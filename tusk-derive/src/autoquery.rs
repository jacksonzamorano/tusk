use proc_macro2::Ident;
use quote::format_ident;
use syn::Fields;

#[derive(Default)]
pub struct AutoqueryParams {
    pub table_name: String,
    pub ignore_keys: Vec<String>,
}

impl AutoqueryParams {
    pub fn from_string(d: String) -> AutoqueryParams {
        let mut params = AutoqueryParams::default();
        for args in d.split("|").filter(|x| x.trim() != "") {
            let split = args.split("=").collect::<Vec<&str>>();
            match split[0].trim() {
                "table_name" => params.table_name = split[1].trim().to_string(),
                "ignore_keys" => {
                    params.ignore_keys = split[1].split(",").map(|x| x.trim().to_string()).collect()
                }
                _ => panic!("Unknown params '{}'", split[0]),
            }
        }
        params
    }
}

pub fn create_insert_fn(
    name: &Ident,
    fields: &Fields,
    params: &AutoqueryParams,
) -> proc_macro2::TokenStream {
    let pg_field_names = fields
        .iter()
        .map(|x| x.ident.as_ref().unwrap().to_string())
        .filter(|x| !params.ignore_keys.contains(x))
        .collect::<Vec<_>>();

    let pg_params = fields
        .iter()
        .filter(|x| pg_field_names.contains(&x.ident.as_ref().unwrap().to_string()))
        .map(|x| {
            let x_name = &x.ident;
            quote! {
                &self.#x_name
            }
        })
        .collect::<Vec<_>>();

    let insert_query = format!(
        "INSERT INTO {} ({}) VALUES ({}) RETURNING *",
        params.table_name,
        pg_field_names.join(","),
        (1..=pg_field_names.len())
            .map(|x| format!("${}", x))
            .collect::<Vec<_>>()
            .join(",")
    );

    quote! {
        pub async fn insert(&self, db: &tusk_rs::PostgresConn) -> #name {
            <#name as tusk_rs::FromSql>::from_postgres(db.query(#insert_query, &[#(#pg_params),*])
                .await
                .unwrap()
                .first()
                .unwrap())
        }
    }
}

pub fn select_query(name: &Ident, fields: &Fields) -> proc_macro2::TokenStream {
    let struct_name = format_ident!("{}Query", name);

    let convs = fields
        .iter()
        .map(|x| {
            let x_ident = &x.ident;
            let x_ident_string = x_ident.clone().unwrap().to_string();
            let x_type = &x.ty;

            quote! {
                pub fn #x_ident(mut self, data: tusk_rs::WhereClause<#x_type>) -> Self {
                    self.data.insert(#x_ident_string, data.into_data());
                    self
                }
            }
        })
        .collect::<Vec<proc_macro2::TokenStream>>();

    quote! {
        pub struct #struct_name {
            data: std::collections::HashMap::<&'static str, tusk_rs::WhereClauseData>
        }
        impl #struct_name {
            pub fn new() -> #struct_name {
                #struct_name { data: std::collections::HashMap::new() }
            }
            #(#convs)*
        }
        impl tusk_rs::QueryObject for #struct_name {
            fn into_params(self) -> std::collections::HashMap::<&'static str, tusk_rs::WhereClauseData> { return self.data }
        }
    }.into()
}

pub fn extras(name: &Ident, fields: &Fields, params: &AutoqueryParams) -> proc_macro2::TokenStream {
    let t_name = &params.table_name;
    let fields_name = format_ident!("{}Fields", name);

    let convs = fields
        .iter()
        .map(|x| {
            let x_ident_string = x.ident.as_ref().unwrap().to_string();
            let mut iter = x_ident_string.chars();
            let ident_name = format_ident!(
                "{}",
                iter.next().unwrap().to_uppercase().to_string() + iter.as_str()
            );
            quote! {
                #ident_name
            }
        })
        .collect::<Vec<proc_macro2::TokenStream>>();

    let cols = fields
        .iter()
        .map(|x| {
            let col_name = x.ident.as_ref().unwrap().to_string();
            quote! {
                #col_name
            }
        })
        .collect::<Vec<_>>();
    let data = fields
        .iter()
        .map(|x| {
            let col_name_ident = &x.ident;
            quote! {
                &self.#col_name_ident
            }
        })
        .collect::<Vec<_>>();

    quote! {
        pub enum #fields_name {
            #(#convs),*
        }
        impl #fields_name {
        }
        impl tusk_rs::TableType for #name {
            fn table_name() -> &'static str {
                return #t_name
            }
        }
        impl tusk_rs::UpdatableObject for #name {
            fn into_params(&self) -> (&[&str], Vec<&(dyn tusk_rs::ToSql + Sync)>) {
                return (
                    &[#(#cols),*],
                    vec![#(#data),*]
                )
            }
        }
    }
    .into()
}
