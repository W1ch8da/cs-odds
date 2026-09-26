use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;

use crate::{
    adapters::inbound::http::{
        cookies::{REFRESH_COOKIE, clear_session, set_session},
        dto::{LoginRequest, RegisterRequest, SessionDto, UserDto},
        extractors::{ApiJson, CurrentUser},
        state::HttpState,
    },
    application::{
        error::{AppError, AppResult},
        ports::inbound::{LoginInput, RegisterCustomerInput},
    },
};

pub async fn register(
    State(state): State<HttpState>,
    jar: CookieJar,
    ApiJson(body): ApiJson<RegisterRequest>,
) -> AppResult<impl IntoResponse> {
    let session = state
        .auth
        .register_customer(RegisterCustomerInput { email: body.email, name: body.name, password: body.password })
        .await?;
    let jar = set_session(jar, &session, state.cookies);
    Ok((StatusCode::CREATED, jar, Json(SessionDto::from(session))))
}

pub async fn login(
    State(state): State<HttpState>,
    jar: CookieJar,
    ApiJson(body): ApiJson<LoginRequest>,
) -> AppResult<impl IntoResponse> {
    let session = state.auth.login(LoginInput { email: body.email, password: body.password }).await?;
    let jar = set_session(jar, &session, state.cookies);
    Ok((jar, Json(SessionDto::from(session))))
}

/// On failure the cookies are cleared too, so the browser stops retrying.
pub async fn refresh(State(state): State<HttpState>, jar: CookieJar) -> Result<impl IntoResponse, Response> {
    let token = jar.get(REFRESH_COOKIE).map(|c| c.value().to_owned());
    let result = match token {
        Some(token) => state.auth.refresh(&token).await,
        None => Err(AppError::Unauthorized),
    };
    match result {
        Ok(session) => {
            let jar = set_session(jar, &session, state.cookies);
            Ok((jar, Json(SessionDto::from(session))))
        }
        Err(err) => Err((clear_session(jar), err).into_response()),
    }
}

pub async fn logout(State(state): State<HttpState>, jar: CookieJar) -> AppResult<impl IntoResponse> {
    if let Some(token) = jar.get(REFRESH_COOKIE) {
        state.auth.logout(token.value()).await?;
    }
    Ok((StatusCode::NO_CONTENT, clear_session(jar)))
}

pub async fn me(State(state): State<HttpState>, CurrentUser(principal): CurrentUser) -> AppResult<Json<UserDto>> {
    Ok(Json(state.auth.current_user(&principal).await?.into()))
}
