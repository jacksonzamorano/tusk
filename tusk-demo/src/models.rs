use tusk_rs::PostgresTable;
use tusk_rs_derive::{FromPostgres, PostgresReadFields, PostgresWriteFields, PostgresWriteable, PostgresJoins};


pub struct RouteData {}

#[derive(FromPostgres, PostgresReadFields,  PostgresWriteFields, PostgresWriteable, PostgresJoins)]
pub struct TestFromPostgres {
	pub username: String,
	pub password: String
}
impl PostgresTable for TestFromPostgres {
    fn table_name() -> &'static str {
        "test"
    }
}
