use uuid::Uuid;

pub type DbResult<T> = Result<T, diesel::result::Error>;

#[derive(Clone, Debug)]
pub struct AuthenticatedUser {
    pub id: Uuid,
    pub email: String,
    pub name: String,
}
