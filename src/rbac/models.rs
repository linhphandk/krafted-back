use chrono::NaiveDateTime;
use diesel::prelude::*;
use uuid::Uuid;

#[derive(Queryable, Selectable, Clone, Debug)]
#[diesel(table_name = crate::schema::roles)]
pub struct Role {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::roles)]
pub struct NewRole {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Queryable, Selectable, Clone, Debug)]
#[diesel(table_name = crate::schema::permissions)]
pub struct Permission {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::permissions)]
pub struct NewPermission {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Queryable, Selectable, Insertable, Clone, Debug)]
#[diesel(table_name = crate::schema::user_roles)]
pub struct UserRole {
    pub user_id: Uuid,
    pub role_id: Uuid,
    pub created_at: NaiveDateTime,
}

#[derive(Queryable, Selectable, Insertable, Clone, Debug)]
#[diesel(table_name = crate::schema::role_permissions)]
pub struct RolePermission {
    pub role_id: Uuid,
    pub permission_id: Uuid,
    pub created_at: NaiveDateTime,
}
