use tusk_rs_derive::{FromPostgres, PostgresReadFields, PostgresWriteFields, PostgresWriteable};


pub struct RouteData {}

#[derive(FromPostgres, PostgresReadFields,  PostgresWriteFields, PostgresWriteable)]
pub struct TestFromPostgres {
	pub username: String,
	pub password: String
}

// #[autoquery(table_name=users)]
// #[derive(ToJson)]
// pub struct NewUser {
//     pub id: Option<i32>,
//     pub email: String,
// }
// impl NewUser {
//     pub async fn all_users(db: &PostgresConn) -> Vec<NewUser> {
//         return SelectQuery::new()
//             .limit(50)
//             .order_by("email", false)
//             .query_all(&db)
//             .await
//             .unwrap_or(vec![])
//     }
// }