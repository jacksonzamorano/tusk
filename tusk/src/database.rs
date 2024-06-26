use deadpool_postgres::{Object, Pool};
use openssl::ssl::{SslConnector, SslMethod};
use postgres_openssl::MakeTlsConnector;
use tokio_postgres::{types::ToSql, NoTls, Row};

use crate::{
    config::DatabaseConfig, query::{PostgresReadable, IntoSyntax}, FromPostgres, PostgresTable, PostgresWrite, PostgresReadFields
};


/// A thin wrapper for the Deadpool Postgres library.
/// Used by Tusk to allocate connections for each route.
#[derive(Clone)]
pub struct Database {
    pool: Pool,
    debug: bool,
}

impl Database {
    /// Create a new database pool.
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
            Some(Database { pool, debug: config.debug })
        } else {
            let pool = cfg.create_pool(None, NoTls).ok()?;
            Some(Database { pool, debug: config.debug })
        }
    }

    /// Gets a connection within the pool.
    pub async fn get_connection(&self) -> Result<DatabaseConnection, deadpool_postgres::PoolError> {
        Ok(DatabaseConnection {
            cn: self.pool.get().await?,
            debug: self.debug,
        })
    }
}

#[derive(Debug)]
pub enum PostgresReadError {
    NoResults,
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
                "42501" => PostgresReadError::PermissionDenied(err.as_db_error().unwrap().table().unwrap().to_string()),
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

#[derive(Debug)]
pub enum PostgresWriteError {
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
                "42501" => PostgresWriteError::PermissionDenied(err.as_db_error().unwrap().table().unwrap().to_string()),
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

pub struct DatabaseConnection {
    cn: Object,
    debug: bool,
}
impl DatabaseConnection {
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

    pub async fn select_all<T: FromPostgres + PostgresReadable + PostgresTable>(
        &self,
        query: &str,
        args: &[&(dyn ToSql + Sync)],
    ) -> Result<Vec<T>, PostgresReadError> {
        if self.debug {
            println!("[DEBUG: QUERY] (select_all) SELECT {} FROM {} {} {}", T::read_fields().as_syntax(T::table_name()), T::table_name(), T::joins().iter().map(|j| j.to_read(T::table_name())).collect::<Vec<String>>().join(" "), query);
            println!("[DEBUG: ARGS] (select_all) Args: {:?}", args);
        }
        Ok(self
            .cn
            .query(
                &format!(
                    "SELECT {} FROM {} {} {}",
                    T::read_fields().as_syntax(T::table_name()),
                    T::table_name(),
                    T::joins().iter().map(|j| j.to_read(T::table_name())).collect::<Vec<String>>().join(" "),
                    query
                ),
                args,
            )
            .await?
            .iter()
            .map(|x| T::from_postgres(x))
            .collect::<Vec<_>>())
    }

    pub async fn select_one<T: FromPostgres + PostgresReadable + PostgresTable>(
        &self,
        query: &str,
        args: &[&(dyn ToSql + Sync)],
    ) -> Result<T, PostgresReadError> {
        if self.debug {
            println!("[DEBUG: QUERY] (select_one) SELECT {} FROM {} {} {}",
                    T::read_fields().as_syntax(T::table_name()),
                    T::table_name(),
                    T::joins().iter().map(|j| j.to_read(T::table_name())).collect::<Vec<String>>().join(" "),
                    query);
            println!("[DEBUG: ARGS] (select_one) Args: {:?}", args);
        }
        self
            .cn
            .query(
                &format!(
                    "SELECT {} FROM {} {} {}",
                    T::read_fields().as_syntax(T::table_name()),
                    T::table_name(),
                    T::joins().iter().map(|j| j.to_read(T::table_name())).collect::<Vec<String>>().join(" "),
                    query
                ),
                args,
            )
            .await?
            .iter()
            .map(|x| T::from_postgres(x))
            .next()
            .ok_or_else(|| PostgresReadError::NoResults)
    }

    pub async fn insert<T: FromPostgres + PostgresTable + PostgresReadFields>(
        &self,
        write: PostgresWrite,
    ) -> Result<T, PostgresWriteError> {
        let (insert_q, insert_a) = write.into_insert(T::table_name());
        if self.debug {
            println!("[DEBUG: QUERY] (insert) {} RETURNING {}", insert_q, T::read_fields().as_syntax(T::table_name()));
            println!("[DEBUG: ARGS] (insert) Args: {:?}", insert_a);
        }
        Ok(self
            .cn
            .query(&format!("{} RETURNING {}", insert_q, T::read_fields().as_syntax(T::table_name())), insert_a.as_slice())
            .await?
            .iter()
            .map(|x| T::from_postgres(x))
            .next()
            .unwrap())
    }

    pub async fn insert_vec<T: FromPostgres + PostgresTable + PostgresReadable>(
        &self,
        write: PostgresWrite,
    ) -> Result<Vec<T>, PostgresWriteError> {
        let (insert_q, insert_a) = write.into_bulk_insert(T::table_name());
        if insert_a.is_empty() {
            return Err(PostgresWriteError::NoRows);
        }
        let temp_table = format!("write_{}", T::table_name());
        let join_str = if !T::joins().is_empty() { T::joins().as_syntax(&temp_table) } else { "".to_string() };
        if self.debug {
            println!("[DEBUG: QUERY] (insert_vec) WITH {} AS ({} RETURNING *) SELECT {} FROM {} {}", temp_table, insert_q, T::read_fields().as_syntax(&temp_table), temp_table, join_str);
            println!("[DEBUG: ARGS] (insert_vec) Args: {:?}", insert_a);
        }
        Ok(self
            .cn
            .query(&format!("WITH {} AS ({} RETURNING *) SELECT {} FROM {} {}", temp_table, insert_q, T::read_fields().as_syntax(&temp_table), temp_table, join_str), insert_a.as_slice())
            .await?
            .iter()
            .map(|x| T::from_postgres(x))
            .collect())
    }

    pub async fn update<T: FromPostgres + PostgresTable + PostgresReadable + std::fmt::Debug>(
        &self,
        write: PostgresWrite,
        condition: &str,
        args: &[&(dyn ToSql + Sync)],
    ) -> Result<T, PostgresWriteError> {
        let temp_table = format!("write_{}", T::table_name());
        let (insert_q, insert_a) = write.into_update(T::table_name(), args.len());
        if self.debug {
            println!("[DEBUG: QUERY] (update) WITH {} AS ({} WHERE {} RETURNING *) SELECT {} FROM {} {}", temp_table, insert_q, condition, T::read_fields().as_syntax(&temp_table), temp_table, T::joins().as_syntax(&temp_table));
            println!("[DEBUG: ARGS] (update) Args: {:?}", [args, insert_a.as_slice()].concat());
        }
        let next = self
            .cn
            .query(
                &format!("WITH {} AS ({} WHERE {} RETURNING *) SELECT {} FROM {} {}",
                    temp_table,
                    insert_q,
                    condition,
                    T::read_fields().as_syntax(&temp_table),
                    temp_table,
                    T::joins().as_syntax(&temp_table)
                ),
                [args, insert_a.as_slice()].concat().as_slice(),
            )
            .await?
            .iter()
            .map(|x| T::from_postgres(x))
            .collect::<Vec<_>>();
        Ok(next.into_iter().next().unwrap())
    }

    pub async fn update_set<T: FromPostgres + PostgresTable + PostgresReadable>(
        &self,
        query: &str,
        args: &[&(dyn ToSql + Sync)],
    ) -> Result<T, PostgresWriteError> {
        let temp_table = format!("write_{}", T::table_name());
        if self.debug {
            println!("[DEBUG: QUERY] (update_set) WITH {} AS (UPDATE {} SET {} RETURNING *) SELECT {} FROM {} {}", temp_table, T::table_name(), query, T::read_fields().as_syntax(&temp_table), temp_table, T::joins().as_syntax(&temp_table));
            println!("[DEBUG: ARGS] (update_set) Args: {:?}", args);
        }
        Ok(self
            .cn
            .query(
                &format!("with {} as (update {} set {} returning *) select {} from {} {}",
                    temp_table,
                    T::table_name(),
                    query,
                    T::read_fields().as_syntax(&temp_table),
                    temp_table,
                    T::joins().as_syntax(&temp_table)
                ),
                args,
            )
            .await?
            .iter()
            .map(|x| T::from_postgres(x))
            .next()
            .unwrap())
    }

    pub async fn delete<T: PostgresTable>(&self, condition: &str, args: &[&(dyn ToSql + Sync)]) {
        if self.debug {
            println!("[DEBUG: QUERY] (delete) DELETE FROM {} {}", T::table_name(), condition);
            println!("[DEBUG: ARGS] (delete) Args: {:?}", args);
        }
        let _ = self.cn.query(
            &format!("DELETE FROM {} {}", T::table_name(), condition),
            args,
        ).await;
    }
}

pub enum DatabaseError {
    Unknown,
    ForeignKey(String),
    NoResults,
}
