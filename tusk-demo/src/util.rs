#[macro_export] macro_rules! auth {
    ($data:ident) => {
     	$data.user.ok_or(RouteError::forbidden("Not authenticated!"))?
    };
}

#[macro_export] macro_rules! json_string {
    ($body:ident, $field:literal) => {
        $body.validate_string($field, &format!("{} is a required field.", $field))?
    };
}

#[macro_export] macro_rules! json_i32 {
    ($body:ident, $field:literal) => {
        $body.validate_i32($field, &format!("{} is a required field.", $field))?
    };
}

#[macro_export] macro_rules! insert_result {
    ($create:expr) => {
        $create.map_err(|x| match x {
            tusk::DatabaseError::Unknown => RouteError::server_error("Cannot create record"),
            tusk::DatabaseError::ForeignKey(key) => RouteError::bad_request(&format!("Value for field {} does not exist", key)),
            _ => tusk::RouteError::server_error("Cannot create record")
        })?
    };
}