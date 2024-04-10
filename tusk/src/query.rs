use tokio_postgres::{types::ToSql, Row};

pub trait IntoSyntax {
    fn as_syntax(&self, local_table: &str) -> String;
}
impl IntoSyntax for &'static [&'static PostgresField] {
    fn as_syntax(&self, local_table: &str) -> String {
        let mut v = self.iter().map(|x| x.into_syntax(local_table)).fold(String::new(), |acc, x| acc + &x + ",");
        v.pop();
        v
    }
}
impl IntoSyntax for &'static [&'static PostgresJoin] {
    fn as_syntax(&self, local_table: &str) -> String {
        let mut v = self.iter().map(|x| x.to_read(local_table)).fold(String::new(), |acc, x| acc + &x + " ");
        v.pop();
        v
    }
}

pub trait FromPostgres {
    fn from_postgres(row: &Row) -> Self;
    fn try_from_postgres(row: &Row) -> Result<Self, FromPostgresError>
    where
        Self: Sized;
}
pub enum FromPostgresError {
    InvalidType(&'static str),
    MissingColumn(&'static str),
}

pub struct PostgresJoin {
    pub join_type: &'static str,
    pub table: &'static str,
    pub local_field: &'static str,
    pub foreign_field: &'static str,
    pub condition: &'static str,
}
impl PostgresJoin {
    pub fn to_read(&self, local_table: &str) -> String {
        format!("{} {} ON {}.{} {} {}.{}", self.join_type, self.table, local_table, self.local_field, self.condition, self.table, self.foreign_field)
    }
    pub fn to_write(&self, local_table: &str) -> String {
        format!("FROM {} WHERE {}.{} {} {}.{}", self.table, local_table, self.local_field, self.condition, self.table, self.foreign_field)
    }
}

pub enum PostgresFieldLocation {
    Local(&'static str),
    Expression(&'static str),
    Join(&'static str, &'static str),
}

pub struct PostgresField {
    pub alias: &'static str,
    pub location: PostgresFieldLocation,
}
impl PostgresField {
    pub fn into_syntax(&self, local_table: &str) -> String {
        format!("{} AS {}", match &self.location {
            PostgresFieldLocation::Local(field) => format!("{}.{}", local_table, field),
            PostgresFieldLocation::Expression(expr) => expr.to_string().replace("{}", local_table),
            PostgresFieldLocation::Join(table, field) => format!("{}.{}", table, field),
        }, self.alias)
    }
}

#[macro_export]
macro_rules! local {
    ($name: literal) => {
        &tusk_rs::PostgresField {
            alias: $name,
            location: tusk_rs::PostgresFieldLocation::Local($name),
        }
    };
}

#[macro_export]
macro_rules! local_as {
    ($name: literal, $alias: literal) => {
        &tusk_rs::PostgresField {
            alias: $alias,
            location: tusk_rs::PostgresFieldLocation::Local($name),
        }
    };
}

#[macro_export]
macro_rules! expression {
    ($expr: literal, $alias: literal) => {
        &tusk_rs::PostgresField {
            alias: $alias,
            location: tusk_rs::PostgresFieldLocation::Expression($expr),
        }
    };
}

#[macro_export]
macro_rules! foreign {
    ($table: literal, $name: literal) => {
        &tusk_rs::PostgresField {
            alias: $name,
            location: tusk_rs::PostgresFieldLocation::Join($table, $name),
        }
    };
}

#[macro_export]
macro_rules! foreign_as {
    ($table: literal, $name: literal, $alias: literal) => {
        &tusk_rs::PostgresField {
            alias: $alias,
            location: tusk_rs::PostgresFieldLocation::Join($table, $name),
        }
    };
}

#[derive(Debug)]
pub struct PostgresWrite {
    pub fields: &'static [&'static str],
    pub arguments: Vec<Box<(dyn ToSql + Sync)>>,
}
impl PostgresWrite {
    pub fn into_insert(&self, table_name: &str) -> (String, Vec<&(dyn ToSql + Sync)>) {
        (
            format!(
                "INSERT INTO {} ({}) VALUES ({})",
                table_name,
                self.fields.join(","),
                (0..self.arguments.len())
                    .map(|x| format!("${}", x + 1))
                    .collect::<Vec<String>>()
                    .join(",")
            ),
            self.arguments
                .iter()
                .map(|x| x.as_ref())
                .collect::<Vec<&(dyn ToSql + Sync)>>(),
        )
    }
    pub fn into_bulk_insert(&self, table_name: &str) -> (String, Vec<&(dyn ToSql + Sync)>) {
        if self.arguments.len() % self.fields.len() != 0 {
            panic!("For a bulk insert, arguments % fields must be 0.")
        }
        let mut arg_groups: Vec<String> = vec![];
        
        for ix in 0..(self.arguments.len() / self.fields.len()) {
            let mut iter_args = vec![];
            for jx in 0..self.fields.len() {
                iter_args.push(format!("${}", ix * self.fields.len() + jx + 1))
            }
            arg_groups.push(format!("({})", iter_args.join(",")));
        }
        (
            format!(
                "INSERT INTO {} ({}) VALUES {}",
                table_name,
                self.fields.join(","),
                arg_groups.join(",")
            ),
            self.arguments
                .iter()
                .map(|x| x.as_ref())
                .collect::<Vec<&(dyn ToSql + Sync)>>(),
        )
    }
    pub fn into_update(
        &self,
        table_name: &str,
        arg_offset: usize,
    ) -> (String, Vec<&(dyn ToSql + Sync)>) {
        if self.fields.len() != self.arguments.len() {
            panic!("Field length must equal argument length")
        }
        (
            format!(
                "UPDATE {} SET {}",
                table_name,
                (0..self.arguments.len())
                    .map(|x| format!("{} = ${}", self.fields[x], x + 1 + arg_offset))
                    .collect::<Vec<String>>()
                    .join(",")
            ),
            self.arguments
                .iter()
                .map(|x| x.as_ref())
                .collect::<Vec<&(dyn ToSql + Sync)>>(),
        )
    }
}

pub trait PostgresTable {
    fn table_name() -> &'static str;
}
pub trait PostgresJoins {
    fn joins() -> &'static [&'static PostgresJoin];
}
pub trait PostgresReadFields {
    fn read_fields() -> &'static [&'static PostgresField];
}
pub trait PostgresReadable: PostgresReadFields + PostgresJoins {}
pub trait PostgresWriteFields {
    fn write_fields() -> &'static [&'static str];
}
pub trait PostgresWriteable: PostgresWriteFields + PostgresJoins {
    fn write(self) -> PostgresWrite;
}
pub trait PostgresBulkWriteable {
    fn into_bulk_write(self) -> PostgresWrite;
}

impl<T: PostgresWriteable + PostgresTable> PostgresBulkWriteable for Vec<T> {
    fn into_bulk_write(self) -> PostgresWrite {
        PostgresWrite {
            arguments: self.into_iter().flat_map(|x| x.write().arguments).collect(),
            fields: T::write_fields(),
        }
    }
}
