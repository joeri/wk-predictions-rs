use super::schema::users;

#[derive(Queryable, Identifiable)]
pub struct User {
    pub id: i32,
    pub email: String,
    pub encrypted_password: String,
    pub slack_handle: Option<String>,
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUser<'a> {
    pub email: &'a str,
    pub encrypted_password: &'a str,
    pub slack_handle: Option<&'a str>,
}
