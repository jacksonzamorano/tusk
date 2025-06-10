use tusk_rs::{
    uuid::Uuid, FromPostgres, PostgresField, PostgresJoin, PostgresJoins, PostgresReadFields,
    foreign_as, local_as,
    PostgresReadable, PostgresTable, ToJson,
};

#[derive(FromPostgres, PostgresReadable, PostgresReadFields, PostgresJoins, ToJson)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
}
impl PostgresTable for Organization {
    fn table_name() -> &'static str {
        "organizations"
    }
}

#[derive(FromPostgres, PostgresReadable, PostgresReadFields, PostgresJoins, ToJson)]
pub struct User {
    pub username: String,
    pub password: String,
    pub organization_id: Uuid,
}
impl PostgresTable for User {
    fn table_name() -> &'static str {
        "users"
    }
}

#[derive(FromPostgres, ToJson, PostgresReadable)]
pub struct UserDirectory {
    pub organization_name: String,
    pub username: String,
}
impl PostgresTable for UserDirectory {
    fn table_name() -> &'static str {
        return "users";
    }
}
impl PostgresReadFields for UserDirectory {
    fn read_fields() -> &'static [&'static PostgresField] {
        &[
            foreign_as!("org_join", "name", "organization_name"),
            local_as!("username", "username"),
        ]
    }
}
impl PostgresJoins for UserDirectory {
    fn joins() -> &'static [&'static PostgresJoin] {
        &[&PostgresJoin {
            join_name: "org_join",
            table: "organizations",
            join_type: "INNER JOIN",
            local_field: "organization_id",
            condition: "=",
            foreign_field: "id",
        }]
    }
}
