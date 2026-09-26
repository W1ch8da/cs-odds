//! Database representations of domain enums. Kept here so the domain stays
//! free of sqlx.

use crate::domain::user::Role;

#[derive(Debug, Clone, Copy, sqlx::Type)]
#[sqlx(type_name = "user_role", rename_all = "snake_case")]
pub enum DbRole {
    Admin,
    Agent,
    Customer,
}

impl From<Role> for DbRole {
    fn from(role: Role) -> Self {
        match role {
            Role::Admin => Self::Admin,
            Role::Agent => Self::Agent,
            Role::Customer => Self::Customer,
        }
    }
}

impl From<DbRole> for Role {
    fn from(role: DbRole) -> Self {
        match role {
            DbRole::Admin => Self::Admin,
            DbRole::Agent => Self::Agent,
            DbRole::Customer => Self::Customer,
        }
    }
}
