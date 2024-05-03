/// Defines a connection to a Postgres server.
pub struct DatabaseConfig {
	pub host: String,
	pub port: i32,
	pub username: String,
	pub password: String,
	pub database: String,
	pub ssl: bool,
    pub debug: bool,
}
impl DatabaseConfig {
	/// Creates a new database connection config.
	/// It is setup by default to connect to localhost,
	/// with the username "postgres" and a blank password.
	pub fn new() -> DatabaseConfig {
		DatabaseConfig {
			host: "localhost".to_string(),
			port: 5432,
			username: "postgres".to_string(),
			password: String::new(),
			database: "postgres".to_string(),
			ssl: false,
            debug: false,
		}
	}

	/// Define the host. Can be chained.
	/// 
	/// # Examples
	///
	/// ```
	/// use tusk_rs::config::DatabaseConfig;
	///
	/// DatabaseConfig::new().username("username").password("password")
	/// ```
	pub fn host<T: AsRef<str>>(mut self, host: T) -> DatabaseConfig {
		self.host = host.as_ref().to_string();
		self
	}

	/// Define the username. Can be chained.
	/// 
	/// # Examples
	///
	/// ```
	/// use tusk_rs::config::DatabaseConfig;
	///
	/// DatabaseConfig::new().username("username").password("password")
	/// ```
	pub fn username<T: AsRef<str>>(mut self, username: T) -> DatabaseConfig {
		self.username = username.as_ref().to_string();
		self
	}

	/// Define the password. Can be chained.
	/// 
	/// # Examples
	///
	/// ```
	/// use tusk_rs::config::DatabaseConfig;
	///
	/// DatabaseConfig::new().username("username").password("password")
	/// ```
	pub fn password<T: AsRef<str>>(mut self, password: T) -> DatabaseConfig {
		self.password = password.as_ref().to_string();
		self
	}

	/// Define the database name. Can be chained.
	/// 
	/// # Examples
	///
	/// ```
	/// use tusk_rs::config::DatabaseConfig;
	///
	/// DatabaseConfig::new().username("username").password("password").database("database")
	/// ```
	pub fn database<T: AsRef<str>>(mut self, database: T) -> DatabaseConfig {
		self.database = database.as_ref().to_string();
		self
	}

	/// Define whether SSL should be used. Can be chained.
	/// 
	/// # Examples
	///
	/// ```
	/// use tusk_rs::config::DatabaseConfig;
	///
	/// DatabaseConfig::new().username("username").password("password").ssl(true)
	/// ```
	pub fn ssl(mut self, ssl: bool) -> DatabaseConfig {
		self.ssl = ssl;
		self
	}

	/// Define the port. Can be chained.
	/// 
	/// # Examples
	///
	/// ```
	/// use tusk_rs::config::DatabaseConfig;
	///
	/// DatabaseConfig::new().username("username").password("password").port(5432)
	/// ```
	pub fn port(mut self, port: i32) -> DatabaseConfig {
		self.port = port;
		self
	}

    /// Define whether debug mode should be used. Can be chained.
    /// 
    /// # Examples
    ///
    /// ```
    /// use tusk_rs::config::DatabaseConfig;
    ///
    /// DatabaseConfig::new().username("username").password("password").debug(true)
    /// ```
    pub fn debug(mut self, debug: bool) -> DatabaseConfig {
        self.debug = debug;
        self
    }
}
impl Default for DatabaseConfig {
	fn default() -> Self {
	    DatabaseConfig::new()
	}
}
