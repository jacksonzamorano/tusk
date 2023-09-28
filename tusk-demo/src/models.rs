use tusk_rs::chrono::Utc;
use tusk_rs_derive::{ToJson, autoquery, FromJson};

pub struct RouteData {

}

#[derive(ToJson, FromJson, Debug)]
#[autoquery("
    'ignore_keys:id'
    select_id select_one [id:i32] 'where id = $1'
    update_id update [id:i32] 'where id = $id'
    select_all select [] ''
    delete_id delete [id:i32] 'where id=$1'
")]
pub struct User {
    pub id: Option<i32>,
    pub name: String,
    pub email: String
}

#[derive(ToJson, FromJson, Debug)]
#[autoquery("
    'ignore_keys:id'
    select_all select [] ''
")]
pub struct Client {
    pub id: Option<i32>,
    pub name: String,
    pub created: tusk_rs::chrono::DateTime<Utc>
}

#[derive(ToJson, FromJson)]
pub struct BulkUserUpload {
    pub users: Option<Vec<User>>
}