use deadpool_postgres::{Object, Pool};
use openssl::ssl::{SslConnector, SslMethod};
use postgres_openssl::MakeTlsConnector;
use tokio_postgres::{NoTls};

use crate::config::DatabaseConfig;

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
    pub async fn get_connection(&self) -> Result<Object, deadpool_postgres::PoolError> {
        self.pool.get().await
    }
}

pub enum DatabaseError {
    Unknown,
    ForeignKey(String),
    NoResults
}