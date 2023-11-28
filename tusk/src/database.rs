use deadpool_postgres::{Object, Pool};
use openssl::ssl::{SslConnector, SslMethod};
use postgres_openssl::MakeTlsConnector;
use tokio_postgres::{types::ToSql, NoTls, Row};

use crate::{
    config::DatabaseConfig, query::PostgresReadable, FromPostgres, PostgresTable, PostgresWrite,
};

/// A thin wrapper for the Deadpool Postgres library.
/// Used by Tusk to allocate connections for each route.
#[derive(Clone)]
pub struct Database {
    pool: Pool,
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
            Some(Database { pool })
        } else {
            let pool = cfg.create_pool(None, NoTls).ok()?;
            Some(Database { pool })
        }
    }

    /// Gets a connection within the pool.
    pub async fn get_connection(&self) -> Result<DatabaseConnection, deadpool_postgres::PoolError> {
        Ok(DatabaseConnection {
            cn: self.pool.get().await?,
        })
    }
}

#[derive(Debug)]
pub enum PostgresReadError {
    NoResults,
    Unknown(tokio_postgres::Error),
    // (Column)
    AmbigiousColumn(String),
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
}
impl DatabaseConnection {
    pub async fn query<T: AsRef<str>>(
        &self,
        query: T,
        args: &[&(dyn ToSql + Sync)],
    ) -> Result<Vec<Row>, tokio_postgres::Error> {
        self.cn.query(query.as_ref(), args).await
    }

    pub async fn select_all<T: FromPostgres + PostgresReadable + PostgresTable>(
        &self,
        query: &str,
        args: &[&(dyn ToSql + Sync)],
    ) -> Result<Vec<T>, PostgresReadError> {
        Ok(self
            .cn
            .query(
                &format!(
                    "SELECT {} FROM {} {} {}",
                    T::read_fields(),
                    T::table_name(),
                    T::required_joins(),
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
        self
            .cn
            .query(
                &format!(
                    "SELECT {} FROM {} {} {}",
                    T::read_fields(),
                    T::table_name(),
                    T::required_joins(),
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

    pub async fn insert<T: FromPostgres + PostgresTable>(
        &self,
        write: PostgresWrite,
    ) -> Result<T, PostgresWriteError> {
        let (insert_q, insert_a) = write.into_insert(T::table_name());
        Ok(self
            .cn
            .query(&format!("{} RETURNING *", insert_q), insert_a.as_slice())
            .await?
            .iter()
            .map(|x| T::from_postgres(x))
            .next()
            .unwrap())
    }

    pub async fn insert_vec<T: FromPostgres + PostgresTable>(
        &self,
        write: PostgresWrite,
    ) -> Result<Vec<T>, PostgresWriteError> {
        let (insert_q, insert_a) = write.into_insert(T::table_name());
        Ok(self
            .cn
            .query(&format!("{} RETURNING *", insert_q), insert_a.as_slice())
            .await?
            .iter()
            .map(|x| T::from_postgres(x))
            .collect())
    }

    pub async fn update<T: FromPostgres + PostgresTable>(
        &self,
        write: PostgresWrite,
        condition: &str,
        args: &[&(dyn ToSql + Sync)],
    ) -> Result<T, PostgresWriteError> {
        let (insert_q, insert_a) = write.into_update(T::table_name(), args.len());
        Ok(self
            .cn
            .query(
                &format!("{} WHERE {} RETURNING *", insert_q, condition),
                [&args, insert_a.as_slice()].concat().as_slice(),
            )
            .await?
            .iter()
            .map(|x| T::from_postgres(x))
            .next()
            .unwrap())
    }

    pub async fn update_set<T: FromPostgres + PostgresTable>(
        &self,
        query: &str,
        args: &[&(dyn ToSql + Sync)],
    ) -> Result<T, PostgresWriteError> {
        Ok(self
            .cn
            .query(
                &format!("UPDATE {} {} RETURNING *", T::table_name(), query),
                args,
            )
            .await?
            .iter()
            .map(|x| T::from_postgres(x))
            .next()
            .unwrap())
    }

    pub async fn delete<T: PostgresTable>(&self, condition: &str, args: &[&(dyn ToSql + Sync)]) {
        _ = self.cn.query(
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
