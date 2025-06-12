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

/// A trait for getting a struct from a Postgres row.
///
/// This can be derived provided that each property also
/// implements `FromPostgres`.
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

/// A struct that defines how Tusk should join
/// tables for you.
pub struct PostgresJoin {
    /// The name of the join. Must be used in foreign_as references!
    pub join_name: &'static str,
    /// The type of join to perform. Examples are INNER JOIN, LEFT JOIN, etc.
    pub join_type: &'static str,
    /// The table to join.
    pub table: &'static str,
    /// The field in the local table to join on.
    pub local_field: &'static str,
    /// The field in the foreign table to join on.
    pub foreign_field: &'static str,
    /// The condition to join on. Use SQL syntax.
    pub condition: &'static str,
}
impl PostgresJoin {
    /// Converts the join to a read statement.
    pub fn to_read(&self, local_table: &str) -> String {
        format!("{} {} {} ON {}.{} {} {}.{}", self.join_type, self.table, self.join_name, local_table, self.local_field, self.condition, self.join_name, self.foreign_field)
    }
    /// Tusk returns the insertered or updated row(s),
    /// so this converts the join to a write statement.
    pub fn to_write(&self, local_table: &str) -> String {
        format!("FROM {} {} WHERE {}.{} {} {}.{}", self.table, local_table, self.join_name, self.local_field, self.condition, self.join_name, self.foreign_field)
    }
}

/// Describes what kind of field this is.
/// For more information, read [`PostgresField`].
pub enum PostgresFieldLocation {
    /// A field on the table itself.
    Local(&'static str),
    /// An expression that can be evaluated.
    Expression(&'static str),
    /// A field on a foreign table.
    /// Syntax is (table, field).
    Join(&'static str, &'static str),
}

/// A struct that symbolizes a field in a Postgres table.
/// This is used for reading from tables.
///
/// Intializing this struct manually is frowned upon.
/// The easiest way to construct a PostgresField is to use
/// the macros provided.
/// - [`local`] for a field on the table itself.
/// - [`local_as`] for a field on the table itself with an alias.
/// - [`expression`] for an expression.
/// - [`foreign`] for a field on a foreign table.
/// - [`foreign_as`] for a field on a foreign table with an alias.
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

/// A macro for creating a local field.
///
/// # Arguments
/// - `$name` - The name of the field, as a &'static str.
#[macro_export]
macro_rules! local {
    ($name: literal) => {
        &tusk_rs::PostgresField {
            alias: $name,
            location: tusk_rs::PostgresFieldLocation::Local($name),
        }
    };
}

/// A macro for creating a local field with an alias.
///
/// # Arguments
/// - `$name` - The name of the field, as a &'static str.
/// - `$alias` - The alias of the field, as a &'static str.
#[macro_export]
macro_rules! local_as {
    ($name: literal, $alias: literal) => {
        &tusk_rs::PostgresField {
            alias: $alias,
            location: tusk_rs::PostgresFieldLocation::Local($name),
        }
    };
}

/// A macro for creating an expression.
///
/// # Arguments
/// - `$expr` - The expression to evaluate, as a &'static str.
/// - `$alias` - The alias of the expression, as a &'static str.
#[macro_export]
macro_rules! expression {
    ($expr: literal, $alias: literal) => {
        &tusk_rs::PostgresField {
            alias: $alias,
            location: tusk_rs::PostgresFieldLocation::Expression($expr),
        }
    };
}

/// A macro for creating a foreign field.
///
/// Using [`foreign_as`] is recommended to prevent conflicts where
/// both a local and foreign field have the same name.
///
/// # Arguments
/// - `$table` - The table to join on, as a &'static str.
/// - `$name` - The name of the field, as a &'static str.
#[macro_export]
macro_rules! foreign {
    ($table: literal, $name: literal) => {
        &tusk_rs::PostgresField {
            alias: $name,
            location: tusk_rs::PostgresFieldLocation::Join($table, $name),
        }
    };
}

/// A macro for creating a foreign field with an alias.
/// 
/// # Arguments
/// - `$table` - The table to join on, as a &'static str.
/// - `$name` - The name of the field, as a &'static str.
/// - `$alias` - The alias of the field, as a &'static str.
#[macro_export]
macro_rules! foreign_as {
    ($table: literal, $name: literal, $alias: literal) => {
        &tusk_rs::PostgresField {
            alias: $alias,
            location: tusk_rs::PostgresFieldLocation::Join($table, $name),
        }
    };
}

/// A struct that contains data to write into
/// a Postgres table.
#[derive(Debug)]
pub struct PostgresWrite {
    /// The fields that will be provided.
    pub fields: &'static [&'static str],
    /// The arguments to insert. This supports either
    /// a single row or multiple rows.
    ///
    /// arguments.len() % fields.len() must always be 0.
    pub arguments: Vec<Box<(dyn ToSql + Sync)>>,
}
impl PostgresWrite {
    /// Convert this write into a regular `INSERT` statement.
    ///
    /// The returned tuple contains the query string and argument slice to pass
    /// to [`DatabaseConnection::query`].
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
    /// Convert this write into a bulk `INSERT` statement capable of inserting
    /// multiple rows.
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
    /// Convert this write into an `UPDATE` statement. `arg_offset` specifies how
    /// many parameters are already bound in the generated query (useful when
    /// combining with a `WHERE` clause).
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

/// A trait for defining a table in Postgres.
/// This is used for determining which table
/// to read/write from. This is required for
/// all Tusk database operations.
pub trait PostgresTable {
    /// The name of the table in Postgres.
    fn table_name() -> &'static str;
}

/// A trait for defining joins in Postgres.
/// This is used for determining how to join
/// tables. This is required for all Tusk
/// database operations.
///
/// For more info on implementing this trait,
/// read the documentation for [`PostgresJoin`].
pub trait PostgresJoins {
    /// The joins to perform.
    fn joins() -> &'static [&'static PostgresJoin];
}

/// A trait for defining fields to read from
/// in Postgres. This is required for all
/// Tusk database operations.
///
/// This may be implemented by deriving [`PostgresJoin`],
/// which will read all fields in the struct.
///
/// For more control (for example to include expression columns),
/// implement this manually. To learn more about manual implementation,
/// read the documentation for [`PostgresField`].
pub trait PostgresReadFields {
    /// The fields to read.
    fn read_fields() -> &'static [&'static PostgresField];
}

/// A trait that declares a struct as readable.
/// Even though there are no methods, this is
/// a stub for future implementations.
///
/// For now, it may be implemented by deriving.
pub trait PostgresReadable: PostgresReadFields + PostgresJoins {}

/// A trait for defining fields to write to
/// in Postgres. This is required for all
/// Tusk database operations.
///
/// Unlike [`PostgresReadFields`], this trait
/// returns static string slices instead of structs.
/// Because the struct data is not needed to perform writes,
/// this improves performance.
pub trait PostgresWriteFields {
    fn write_fields() -> &'static [&'static str];
}

/// A trait that declares a struct as writeable.
/// Do not manually implement this. Instead, implement
/// [`PostgresWriteFields`] and [`PostgresJoins`], and derive
/// this trait.
pub trait PostgresWriteable: PostgresWriteFields + PostgresJoins {
    fn write(self) -> PostgresWrite;
}

/// A trait for defining a struct as bulk writeable.
/// This is typically defined on collections of structs.
/// Tusk includes a default implementation for Vec<T> where
/// T implements [`PostgresWriteable`].
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
