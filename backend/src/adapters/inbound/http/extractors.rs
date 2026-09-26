use axum::{
    extract::{FromRequest, FromRequestParts, rejection::JsonRejection},
    http::{header::AUTHORIZATION, request::Parts},
};
use axum_extra::extract::CookieJar;

use super::{cookies::ACCESS_COOKIE, state::HttpState};
use crate::application::{error::AppError, principal::Principal};

/// The signed-in caller. Reads the access token from the auth cookie, or from
/// an `Authorization: Bearer` header for API clients. Rejects with 401.
pub struct CurrentUser(pub Principal);

impl FromRequestParts<HttpState> for CurrentUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &HttpState) -> Result<Self, Self::Rejection> {
        let bearer = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(str::to_owned);
        let token = bearer
            .or_else(|| CookieJar::from_headers(&parts.headers).get(ACCESS_COOKIE).map(|c| c.value().to_owned()))
            .ok_or(AppError::Unauthorized)?;
        Ok(Self(state.auth.authenticate(&token)?))
    }
}

/// `Json` that reports malformed bodies in the standard error shape.
#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(AppError))]
pub struct ApiJson<T>(pub T);

impl From<JsonRejection> for AppError {
    fn from(rejection: JsonRejection) -> Self {
        Self::Validation(format!("The request body is invalid: {}", rejection.body_text()))
    }
}
