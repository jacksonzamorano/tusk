use tokio_postgres::{types::ToSql, Row};

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
                iter_args.push(format!("${}", ix * self.fields.len() + jx))
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
pub trait PostgresReadFields {
    fn read_fields() -> &'static str;
}
pub trait PostgresReadable: PostgresReadFields {
    fn required_joins() -> &'static str;
}
pub trait PostgresWriteFields {
    fn write_fields() -> &'static [&'static str];
}
pub trait PostgresWriteable: PostgresWriteFields {
    fn write(self) -> PostgresWrite;
}
pub trait PostgresBulkWriteable {
    fn into_bulk_write(self) -> PostgresWrite;
}

impl<T: PostgresWriteable> PostgresBulkWriteable for Vec<T> {
    fn into_bulk_write(self) -> PostgresWrite {
        PostgresWrite {
            arguments: self.into_iter().flat_map(|x| x.write().arguments).collect(),
            fields: T::write_fields()
        }
    }
}
