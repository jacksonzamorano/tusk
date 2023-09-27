use tusk_rs_derive::{ToJson, autoquery};

pub struct RouteData {

}

#[derive(ToJson, Debug)]
#[autoquery("
    'ignore_keys:id'
    select_id select_one [id:i32] 'where id = $1'
    update_id update [id:i32] 'where id = $id'
    select_all select [] ''
")]
pub struct User {
    pub id: Option<i32>,
    pub name: String,
    pub email: String
}