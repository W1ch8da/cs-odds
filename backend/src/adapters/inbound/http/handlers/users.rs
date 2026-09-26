use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};

use crate::{
    adapters::inbound::http::{
        dto::{CreateUserRequest, ListUsersQuery, UserDto},
        extractors::{ApiJson, CurrentUser},
        state::HttpState,
    },
    application::{error::AppResult, ports::inbound::CreateStaffInput},
    domain::user::Role,
};

pub async fn list(
    State(state): State<HttpState>,
    CurrentUser(actor): CurrentUser,
    Query(query): Query<ListUsersQuery>,
) -> AppResult<Json<Vec<UserDto>>> {
    let role = query.role.as_deref().map(str::parse::<Role>).transpose()?;
    let users = state.users.list_users(&actor, role).await?;
    Ok(Json(users.into_iter().map(Into::into).collect()))
}

pub async fn create(
    State(state): State<HttpState>,
    CurrentUser(actor): CurrentUser,
    ApiJson(body): ApiJson<CreateUserRequest>,
) -> AppResult<(StatusCode, Json<UserDto>)> {
    let role = body.role.parse::<Role>()?;
    let user = state
        .users
        .create_staff(&actor, CreateStaffInput { email: body.email, name: body.name, password: body.password, role })
        .await?;
    Ok((StatusCode::CREATED, Json(user.into())))
}

/// Agents and admins, for the assignee picker.
pub async fn agents(State(state): State<HttpState>, CurrentUser(actor): CurrentUser) -> AppResult<Json<Vec<UserDto>>> {
    let staff = state.users.list_assignable(&actor).await?;
    Ok(Json(staff.into_iter().map(Into::into).collect()))
}
