use proc_macro2::Ident;
use quote::format_ident;
use std::{collections::HashMap, matches};
use syn::Fields;

pub fn create_insert_fn(
    name: &Ident,
    fields: &Fields,
    table_params: &TableParams,
) -> proc_macro2::TokenStream {
    let pg_field_names = fields
        .iter()
        .map(|x| x.ident.as_ref().unwrap().to_string())
        .filter(|x| !table_params.ignore_keys.contains(x))
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
        table_params.table_name,
        pg_field_names.join(","),
        (1..=pg_field_names.len())
            .map(|x| format!("${}", x))
            .collect::<Vec<_>>()
            .join(",")
    );

    quote! {
        pub async fn insert(&self, db: &tusk_rs::PostgresConn) -> #name {
            #name::from_postgres(db.query(#insert_query, &[#(#pg_params),*])
                .await
                .unwrap()
                .first()
                .unwrap())
        }
    }
}

#[derive(Debug)]
pub struct AutoQueryParser {
    pub queries: Vec<QueryDefinition>,
    pub params: TableParams,
}

impl AutoQueryParser {
    pub fn parse(args: String, default_table: String) -> AutoQueryParser {
        let mut params = TableParams {
            table_name: default_table,
            ..Default::default()
        };
        let mut queries = Vec::new();

        let lines = args
            .split('\n')
            .filter(|x| x.len() > 1)
            .map(|x| x[1..x.len() - 1].trim());
        for l in lines {
            if l.starts_with('\'') {
                // We have an option
                params.add_param(l[1..l.len()].to_string())
            } else if !l.is_empty() {
                // We have a query
                queries.push(QueryDefinition::parse(l.trim().to_string()));
            }
        }

        AutoQueryParser { queries, params }
    }

    pub fn into_token_stream(
        self,
        struct_name: &Ident,
        fields: &[String],
    ) -> (proc_macro2::TokenStream, TableParams) {
        let qs = self
            .queries
            .iter()
            .map(|x| x.generate(struct_name, &self.params, fields))
            .collect::<Vec<_>>();
        (
            quote! {
                #(#qs)*
            },
            self.params,
        )
    }
}

#[derive(Default, Debug)]
pub struct TableParams {
    pub ignore_keys: Vec<String>,
    pub table_name: String,
}

impl TableParams {
    pub fn add_param(&mut self, data: String) {
        let split = data.split(':').map(|x| x.trim()).collect::<Vec<_>>();
        match split[0] {
            "ignore_keys" => {
                self.ignore_keys = split[1].split(',').map(|x| x.to_string()).collect()
            }
            _ => panic!("Unknown parameter {}", split[0]),
        }
    }
}

#[derive(Debug)]
pub struct QueryDefinition {
    query_name: String,
    query_type: QueryType,
    query_args: Vec<String>,
    query_contents: String,
    options: HashMap<String, String>,
}
impl QueryDefinition {
    pub fn parse(data: String) -> QueryDefinition {
        let mut parse_stage = QueryDefinitionParseStage::Name;

        let mut query_name = String::new();
        let mut temp_query_type = String::new();
        let mut query_args = vec![String::new()];
        let mut query_contents = String::new();

        for l in data.chars() {
            match parse_stage {
                QueryDefinitionParseStage::Name => {
                    if l == ' ' {
                        parse_stage = QueryDefinitionParseStage::Type;
                        continue;
                    }
                    query_name.push(l);
                }
                QueryDefinitionParseStage::Type => {
                    if l == '[' {
                        parse_stage = QueryDefinitionParseStage::Args;
                        continue;
                    }
                    temp_query_type.push(l);
                }
                QueryDefinitionParseStage::Args => {
                    if l == ']' {
                        parse_stage = QueryDefinitionParseStage::Contents;
                        continue;
                    } else if l == ',' {
                        query_args.push(String::new());
                    } else if l != ' ' {
                        query_args.last_mut().unwrap().push(l);
                    }
                }
                QueryDefinitionParseStage::Contents => {
                    if l == '(' {
                        parse_stage = QueryDefinitionParseStage::Params;
                        continue;
                    } else if (!query_contents.is_empty() || l != ' ') && l != '\'' {
                        query_contents.push(l);
                    }
                }
                QueryDefinitionParseStage::Params => {}
            }
        }

        QueryDefinition {
            query_name,
            query_type: QueryType::from_string(temp_query_type.trim()),
            query_args,
            query_contents,
            options: HashMap::new(),
        }
    }

    pub fn generate(
        &self,
        struct_ident: &Ident,
        params: &TableParams,
        struct_fields: &[String],
    ) -> proc_macro2::TokenStream {
        let fn_name = format_ident!("{}", self.query_name);

        let fn_args = self
            .query_args
            .iter()
            .filter(|x| !x.is_empty())
            .map(|x| {
                let spl = x.split(':').collect::<Vec<_>>();
                let name = format_ident!("{}", spl[0]);
                let typ = format_ident!("{}", spl[1]);
                quote! {
                    #name: #typ
                }
            })
            .collect::<Vec<_>>();
        let fn_args_name = self
            .query_args
            .iter()
            .filter(|x| !x.is_empty())
            .map(|x| format_ident!("{}", x.split(':').collect::<Vec<_>>()[0]))
            .collect::<Vec<_>>();

        let preface =
            self.query_type
                .generate_preface(params, &self.options, struct_fields, struct_ident);
        let translated_params = self
            .query_args
            .iter()
            .enumerate()
            .map(|(ix, x)| {
                (
                    x.split(':').collect::<Vec<_>>()[0],
                    ix + preface.arg_count + 1,
                )
            })
            .collect::<Vec<_>>();

        let mut query_modified = self.query_contents.clone();
        for (name, id) in translated_params {
            query_modified = query_modified.replace(&format!("${}", name), &format!("${}", id))
        }

        let query = format!("{} {}", preface.query, query_modified);
        let preface_args = preface.args;
        let post_query = preface.post_query;
        let return_type = preface.return_type;

        let s_param = if self.query_type.require_self() {
            quote! { &self, }
        } else {
            quote! {}
        };

        quote! {
            pub async fn #fn_name(#s_param db: &tusk_rs::PostgresConn, #(#fn_args),*) -> #return_type {
                db.query(#query, &[#preface_args #(&#fn_args_name),*])
                    .await #post_query
            }
        }
    }
}
#[derive(Debug)]
enum QueryDefinitionParseStage {
    Name,
    Type,
    Args,
    Contents,
    Params,
}

#[derive(Debug)]
pub enum QueryType {
    Select,
    SelectOne,
    Update,
    Delete,
}
impl QueryType {
    fn from_string(data: &str) -> QueryType {
        match data {
            "select" => QueryType::Select,
            "select_one" => QueryType::SelectOne,
            "update" => QueryType::Update,
            "delete" => QueryType::Delete,
            _ => panic!("Unknown query type '{}'", data),
        }
    }

    fn require_self(&self) -> bool {
        matches!(self, Self::Update)
    }

    fn generate_preface(
        &self,
        table_params: &TableParams,
        _options: &HashMap<String, String>,
        fields: &[String],
        type_ident: &Ident,
    ) -> QueryPreface {
        match self {
            QueryType::Select => QueryPreface {
                query: format!("SELECT * FROM {}", table_params.table_name),
                return_type: quote! {Vec<#type_ident>},
                post_query: quote! {
                    .unwrap().iter().map(|x| #type_ident::from_postgres(x)).collect()
                },
                arg_count: 0,
                args: quote! {},
            },
            QueryType::SelectOne => QueryPreface {
                query: format!("SELECT * FROM {}", table_params.table_name),
                return_type: quote! {Option<#type_ident>},
                post_query: quote! {
                    .unwrap().iter().map(|x| #type_ident::from_postgres(x)).next()
                },
                arg_count: 0,
                args: quote! {},
            },
            QueryType::Update => {
                let fields = fields
                    .iter()
                    .map(|x| format_ident!("{}", x))
                    .collect::<Vec<_>>();
                QueryPreface {
                    query: format!(
                        "UPDATE {} SET {}",
                        table_params.table_name,
                        fields
                            .iter()
                            .enumerate()
                            .map(|(ix, x)| format!("{} = ${}", x, ix + 1))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    return_type: quote! {#type_ident},
                    post_query: quote! {
                        .unwrap().iter().map(|x| #type_ident::from_postgres(x)).next().unwrap()
                    },
                    arg_count: fields.len(),
                    args: quote! {
                        #(&self.#fields),*,
                    },
                }
            }
            QueryType::Delete => QueryPreface {
                query: format!("DELETE FROM {}", table_params.table_name),
                return_type: quote! {()},
                post_query: quote! {;},
                arg_count: 0,
                args: quote! {},
            },
        }
    }
}

#[derive(Debug)]
struct QueryPreface {
    query: String,
    post_query: proc_macro2::TokenStream,
    return_type: proc_macro2::TokenStream,
    arg_count: usize,
    args: proc_macro2::TokenStream,
}
