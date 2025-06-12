use deadpool_postgres::{Object, Pool};
use openssl::ssl::{SslConnector, SslMethod};
use postgres_openssl::MakeTlsConnector;
use tokio_postgres::{types::ToSql, NoTls, Row};

use crate::{
    config::DatabaseConfig,
    query::{IntoSyntax, PostgresReadable},
    FromPostgres, PostgresReadFields, PostgresTable, PostgresWrite, RouteError,
};

/// Convenience macro used when fetching a single record from the database.
///
/// If the provided expression evaluates to `None` a [`RouteError::not_found`]
/// is returned automatically. This allows easily bubbling a not found error
/// when using [`QueryBuilder::get`] or custom queries.
#[macro_export]
macro_rules! expect {
    ($query:expr) => {
        $query.ok_or_else(|| RouteError::not_found("The record you requested was not found."))?
    };
    ($msg:literal, $query:expr) => {
        $query.ok_or_else(|| RouteError::not_found($msg))?
    };
}

/// Similar to [`expect`], but formats the error message with the supplied
/// object name for a friendlier response.
#[macro_export]
macro_rules! expect_obj {
    ($obj:literal, $query:expr) => {
        $query.ok_or_else(|| {
            RouteError::not_found(&format!("The {} you requested was not found.", $obj))
        })?
    };
}

/// A thin wrapper around [`deadpool_postgres`] used by Tusk.
///
/// The [`Database`] type manages a connection pool for your application and is
/// created through [`Database::new`]. Connections are retrieved via
/// [`Database::get_connection`] and passed into route handlers through the
/// [`Request`](crate::Request) type.
#[derive(Clone)]
pub struct Database {
    pool: Pool,
    debug: bool,
}

impl Database {
    /// Create a new connection pool from the provided [`DatabaseConfig`].
    ///
    /// Returns `None` if the pool could not be created.
    pub async fn new(config: DatabaseConfig) -> Option<Database> {
        let mut cfg = deadpool_postgres::Config::new();
        cfg.user = Some(config.username);
        cfg.password = Some(config.password);
        cfg.host = Some(config.host);
        cfg.dbname = Some(config.database);

        if config.ssl {
            let mut builder = SslConnector::builder(SslMethod::tls()).ok()?;
            let _ = builder.set_ca_file("/etc/ssl/cert.pem");
            let connector = MakeTlsConnector::new(builder.build());
            let pool = cfg.create_pool(None, connector).ok()?;
            Some(Database {
                pool,
                debug: config.debug,
            })
        } else {
            let pool = cfg.create_pool(None, NoTls).ok()?;
            Some(Database {
                pool,
                debug: config.debug,
            })
        }
    }

    /// Retrieve a [`DatabaseConnection`] from the pool.
    ///
    /// This should be called for every incoming request and the returned
    /// connection passed to your route handlers.
    pub async fn get_connection(&self) -> Result<DatabaseConnection, deadpool_postgres::PoolError> {
        Ok(DatabaseConnection {
            cn: self.pool.get().await?,
            debug: self.debug,
        })
    }
}

/// Errors that may occur when reading from Postgres.
#[derive(Debug)]
pub enum PostgresReadError {
    Unknown(tokio_postgres::Error),
    // (Column)
    AmbigiousColumn(String),
    // (Table)
    PermissionDenied(String),
}
impl PostgresReadError {
    pub fn from_pg_err(err: tokio_postgres::Error) -> PostgresReadError {
        dbg!(&err);
        if let Some(code) = err.code() {
            match code.code() {
                "42702" => PostgresReadError::AmbigiousColumn(
                    err.as_db_error()
                        .unwrap()
                        .message()
                        .split('\"')
                        .nth(1)
                        .unwrap()
                        .to_string(),
                ),
                "42501" => PostgresReadError::PermissionDenied(
                    err.as_db_error().unwrap().table().unwrap().to_string(),
                ),
                _ => PostgresReadError::Unknown(err),
            }
        } else {
            PostgresReadError::Unknown(err)
        }
    }
}
impl From<tokio_postgres::Error> for PostgresReadError {
    fn from(value: tokio_postgres::Error) -> Self {
        PostgresReadError::from_pg_err(value)
    }
}
impl From<PostgresReadError> for RouteError {
    fn from(value: PostgresReadError) -> Self {
        dbg!(&value);
        RouteError::bad_request("An error occurred and your request could not be fullfilled.")
    }
}

/// Errors that may occur when writing to Postgres.
#[derive(Debug)]
pub enum PostgresWriteError {
    NoWhereProvided,
    InsertValueCountMismatch,
    // (Constraint, Detail)
    UniqueConstraintViolation(String, String),
    // (Column)
    NotNullConstraintViolation(String),
    // (Table)
    PermissionDenied(String),
    NoRows,
    Unknown(tokio_postgres::Error),
}
impl PostgresWriteError {
    pub fn from_pg_err(err: tokio_postgres::Error) -> PostgresWriteError {
        dbg!(&err);
        if let Some(code) = err.code() {
            match code.code() {
                "42601" => PostgresWriteError::InsertValueCountMismatch,
                "23505" => PostgresWriteError::UniqueConstraintViolation(
                    err.as_db_error().unwrap().constraint().unwrap().to_string(),
                    err.as_db_error().unwrap().detail().unwrap().to_string(),
                ),
                "23502" => PostgresWriteError::NotNullConstraintViolation(
                    err.as_db_error().unwrap().column().unwrap().to_string(),
                ),
                "42501" => PostgresWriteError::PermissionDenied(
                    err.as_db_error().unwrap().table().unwrap().to_string(),
                ),
                _ => PostgresWriteError::Unknown(err),
            }
        } else {
            PostgresWriteError::Unknown(err)
        }
    }
}
impl From<tokio_postgres::Error> for PostgresWriteError {
    fn from(value: tokio_postgres::Error) -> Self {
        PostgresWriteError::from_pg_err(value)
    }
}
impl From<PostgresWriteError> for RouteError {
    fn from(value: PostgresWriteError) -> Self {
        dbg!(&value);
        RouteError::bad_request("An error occurred and your request could not be fullfilled.")
    }
}

/// Trait used by [`QueryBuilder`] to represent column identifiers.
pub trait ColumnKeys {
    /// Return the column name as it exists in the database.
    fn name(&self) -> &'static str;
}

/// Helper trait for models that can be used with [`QueryBuilder`].
pub trait Columned: PostgresReadable + PostgresReadFields + FromPostgres + PostgresTable {
    type ReadKeys: ColumnKeys;
    type WriteKeys: ColumnKeys;
}

enum QueryComponent<T: ColumnKeys> {
    Filter(QueryParam<T>),
    And,
    Or,
    Limit(i32),
    Offset(i32),
}
impl<T: ColumnKeys> QueryComponent<T> {
    fn to_query(&self) -> String {
        match self {
            Self::Filter(param) => {
                format!("{} {} ${}", param.key.name(), param.condition, param.arg)
            }
            Self::And => "AND".to_string(),
            Self::Or => "OR".to_string(),
            Self::Limit(limit) => format!("LIMIT {}", limit),
            Self::Offset(offset) => format!("OFFSET {}", offset),
        }
    }
    fn is_filter(&self) -> bool {
        matches!(self, Self::Filter(_))
    }
}

struct QueryParam<T: ColumnKeys> {
    key: T,
    condition: String,
    arg: usize,
}

/// Builder used to easily construct simple `SELECT`, `UPDATE` and `DELETE` queries.
pub struct QueryBuilder<'a, T: Columned> {
    set: Vec<String>,
    filters: Vec<QueryComponent<T::ReadKeys>>,
    args: Vec<&'a (dyn ToSql + Sync)>,
    force: bool,
}
impl<T: Columned> Default for QueryBuilder<'_, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, T: Columned> QueryBuilder<'a, T> {
    /// Create a new [`QueryBuilder`].
    pub fn new() -> QueryBuilder<'a, T> {
        QueryBuilder {
            set: Vec::new(),
            filters: Vec::new(),
            args: Vec::new(),
            force: false,
        }
    }
    /// Set a column to a value for an upcoming [`update_one`](QueryBuilder::update_one)
    /// or [`update_many`](QueryBuilder::update_many) call.
    pub fn set(mut self, key: T::WriteKeys, val: &'a (dyn ToSql + Sync)) -> Self {
        self.set
            .push(format!("{} = ${}", key.name(), self.args.len() + 1));
        self.args.push(val);
        self
    }
    /// Filter results where the provided column equals the value.
    pub fn where_eq(mut self, key: T::ReadKeys, val: &'a (dyn ToSql + Sync)) -> Self {
        self.filters.push(QueryComponent::Filter(QueryParam {
            key,
            condition: "=".to_string(),
            arg: self.args.len() + 1,
        }));
        self.args.push(val);
        self
    }
    /// Filter results where the provided column does not equal the value.
    pub fn where_ne(mut self, key: T::ReadKeys, val: &'a (dyn ToSql + Sync)) -> Self {
        self.filters.push(QueryComponent::Filter(QueryParam {
            key,
            condition: "<>".to_string(),
            arg: self.args.len() + 1,
        }));
        self.args.push(val);
        self
    }
    /// Append an `AND` to the where clause.
    pub fn and(mut self) -> Self {
        self.filters.push(QueryComponent::And);
        self
    }
    /// Append an `OR` to the where clause.
    pub fn or(mut self) -> Self {
        self.filters.push(QueryComponent::Or);
        self
    }
    /// Apply a limit
    pub fn limit(mut self, val: i32) -> Self {
        self.filters.push(QueryComponent::Limit(val));
        self
    }
    /// Apply an offset
    pub fn offset(mut self, val: i32) -> Self {
        self.filters.push(QueryComponent::Offset(val));
        self
    }
    /// By default, write operations without a "WHERE" clause
    /// will be rejected. Call this function
    /// to force it to work.
    pub fn force(mut self) -> Self {
        self.force = true;
        self
    }

    fn build_trail(&self) -> String {
        if !self.filters.is_empty() {
            format!(
                "{} {}",
                if self.filters.first().map(|x| x.is_filter()).unwrap_or(false) {
                    "WHERE "
                } else {
                    ""
                },
                self.filters
                    .iter()
                    .map(|x| x.to_query())
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        } else {
            String::new()
        }
    }

    /// Select one will fetch an object from the database, and return an Option indicating whether it's
    /// been found.
    pub async fn get(self, db: &DatabaseConnection) -> Result<Option<T>, PostgresReadError> {
        Ok(db
            .query(
                &format!(
                    "SELECT {} FROM {} {} {}",
                    T::read_fields().as_syntax(T::table_name()),
                    T::table_name(),
                    T::joins()
                        .iter()
                        .map(|j| j.to_read(T::table_name()))
                        .collect::<Vec<String>>()
                        .join(" "),
                    self.build_trail()
                ),
                &self.args,
            )
            .await?
            .iter()
            .map(|x| T::from_postgres(x))
            .next())
    }
    /// Select all will fetch many objects from the database, and return a Vec. If no options are
    /// found, an empty Vec is returned.
    pub async fn select_all(self, db: &DatabaseConnection) -> Result<Vec<T>, PostgresReadError> {
        Ok(db
            .query(
                &format!(
                    "SELECT {} FROM {} {} {}",
                    T::read_fields().as_syntax(T::table_name()),
                    T::table_name(),
                    T::joins()
                        .iter()
                        .map(|j| j.to_read(T::table_name()))
                        .collect::<Vec<String>>()
                        .join(" "),
                    self.build_trail(),
                ),
                &self.args,
            )
            .await?
            .iter()
            .map(|x| T::from_postgres(x))
            .collect())
    }
    /// Update rows and return all updated records.
    pub async fn update_many(self, db: &DatabaseConnection) -> Result<Vec<T>, PostgresWriteError> {
        let temp_table = format!("write_{}", T::table_name());
        if self.filters.is_empty() && !self.force {
            return Err(PostgresWriteError::NoWhereProvided);
        }
        Ok(db
            .query(
                &format!(
                    "WITH {} AS (UPDATE {} SET {} {} RETURNING *) SELECT {} FROM {} {}",
                    temp_table,
                    T::table_name(),
                    self.set.join(", "),
                    self.build_trail(),
                    T::read_fields().as_syntax(&temp_table),
                    temp_table,
                    T::joins().as_syntax(&temp_table),
                ),
                self.args.as_slice(),
            )
            .await?
            .iter()
            .map(|x| T::from_postgres(x))
            .collect())
    }
    /// Update row and return first updated record.
    pub async fn update_one(
        self,
        db: &DatabaseConnection,
    ) -> Result<Option<T>, PostgresWriteError> {
        let temp_table = format!("write_{}", T::table_name());
        if self.filters.is_empty() && !self.force {
            return Err(PostgresWriteError::NoWhereProvided);
        }
        Ok(db
            .query(
                &format!(
                    "WITH {} AS (UPDATE {} SET {} {} RETURNING *) SELECT {} FROM {} {}",
                    temp_table,
                    T::table_name(),
                    self.set.join(", "),
                    self.build_trail(),
                    T::read_fields().as_syntax(&temp_table),
                    temp_table,
                    T::joins().as_syntax(&temp_table),
                ),
                self.args.as_slice(),
            )
            .await?
            .iter()
            .map(|x| T::from_postgres(x))
            .next())
    }
    /// Delete rows matching the provided condition.
    pub async fn delete(&self, db: &DatabaseConnection) -> Result<(), PostgresWriteError> {
        _ = db
            .query(
                &format!("DELETE FROM {} {}", T::table_name(), self.build_trail()),
                &self.args,
            )
            .await?;
        Ok(())
    }
}

/// Wrapper around a single pooled database connection.
///
/// Instances of this type are passed to route handlers and expose helper
/// methods for common CRUD operations.
pub struct DatabaseConnection {
    cn: Object,
    debug: bool,
}
impl DatabaseConnection {
    /// Execute a raw SQL query and return the resulting rows.
    pub async fn query<T: AsRef<str>>(
        &self,
        query: T,
        args: &[&(dyn ToSql + Sync)],
    ) -> Result<Vec<Row>, tokio_postgres::Error> {
        if self.debug {
            println!("[DEBUG: QUERY] {}", query.as_ref());
            println!("[DEBUG: ARGS] Args: {:?}", args);
        }
        self.cn.query(query.as_ref(), args).await
    }
    /// Insert a single record and return the inserted row.
    pub async fn insert<T: FromPostgres + PostgresTable + PostgresReadFields>(
        &self,
        write: PostgresWrite,
    ) -> Result<T, PostgresWriteError> {
        let (insert_q, insert_a) = write.into_insert(T::table_name());
        if self.debug {
            println!(
                "[DEBUG: QUERY] (insert) {} RETURNING {}",
                insert_q,
                T::read_fields().as_syntax(T::table_name())
            );
            println!("[DEBUG: ARGS] (insert) Args: {:?}", insert_a);
        }
        Ok(self
            .cn
            .query(
                &format!(
                    "{} RETURNING {}",
                    insert_q,
                    T::read_fields().as_syntax(T::table_name())
                ),
                insert_a.as_slice(),
            )
            .await?
            .iter()
            .map(|x| T::from_postgres(x))
            .next()
            .unwrap())
    }

    /// Insert many records and return the inserted rows.
    pub async fn insert_vec<T: FromPostgres + PostgresTable + PostgresReadable>(
        &self,
        write: PostgresWrite,
    ) -> Result<Vec<T>, PostgresWriteError> {
        let (insert_q, insert_a) = write.into_bulk_insert(T::table_name());
        if insert_a.is_empty() {
            return Err(PostgresWriteError::NoRows);
        }
        let temp_table = format!("write_{}", T::table_name());
        let join_str = if !T::joins().is_empty() {
            T::joins().as_syntax(&temp_table)
        } else {
            "".to_string()
        };
        if self.debug {
            println!(
                "[DEBUG: QUERY] (insert_vec) WITH {} AS ({} RETURNING *) SELECT {} FROM {} {}",
                temp_table,
                insert_q,
                T::read_fields().as_syntax(&temp_table),
                temp_table,
                join_str
            );
            println!("[DEBUG: ARGS] (insert_vec) Args: {:?}", insert_a);
        }
        Ok(self
            .cn
            .query(
                &format!(
                    "WITH {} AS ({} RETURNING *) SELECT {} FROM {} {}",
                    temp_table,
                    insert_q,
                    T::read_fields().as_syntax(&temp_table),
                    temp_table,
                    join_str
                ),
                insert_a.as_slice(),
            )
            .await?
            .iter()
            .map(|x| T::from_postgres(x))
            .collect())
    }
}

/// Generic errors that may occur during database operations.
pub enum DatabaseError {
    Unknown,
    ForeignKey(String),
    NoResults,
}
