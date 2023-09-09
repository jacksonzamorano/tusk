pub struct DatabaseConfig {
	pub host: String,
	pub port: i32,
	pub username: String,
	pub password: String,
	pub database: String,
	pub ssl: bool
}
impl DatabaseConfig {
	pub fn new() -> DatabaseConfig {
		DatabaseConfig {
			host: "localhost".to_string(),
			port: 5432,
			username: "postgres".to_string(),
			password: String::new(),
			database: "postgres".to_string(),
			ssl: false
		}
	}
	pub fn host<T: AsRef<str>>(mut self, host: T) -> DatabaseConfig {
		self.host = host.as_ref().to_string();
		self
	}
	pub fn username<T: AsRef<str>>(mut self, username: T) -> DatabaseConfig {
		self.username = username.as_ref().to_string();
		self
	}
	pub fn password<T: AsRef<str>>(mut self, password: T) -> DatabaseConfig {
		self.password = password.as_ref().to_string();
		self
	}
	pub fn database<T: AsRef<str>>(mut self, database: T) -> DatabaseConfig {
		self.database = database.as_ref().to_string();
		self
	}
	pub fn ssl(mut self, ssl: bool) -> DatabaseConfig {
		self.ssl = ssl;
		self
	}
	pub fn port(mut self, port: i32) -> DatabaseConfig {
		self.port = port;
		self
	}
}