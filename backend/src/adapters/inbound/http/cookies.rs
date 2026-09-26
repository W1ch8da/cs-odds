use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::{DateTime, Utc};

use super::state::CookieSettings;
use crate::application::ports::inbound::AuthSession;

pub const ACCESS_COOKIE: &str = "cs_access";
pub const REFRESH_COOKIE: &str = "cs_refresh";

fn build(name: &'static str, value: String, expires_at: DateTime<Utc>, settings: CookieSettings) -> Cookie<'static> {
    let max_age = (expires_at - Utc::now()).num_seconds().max(0);
    Cookie::build((name, value))
        .http_only(true)
        .secure(settings.secure)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(time::Duration::seconds(max_age))
        .build()
}

/// Both tokens live in httpOnly cookies so page scripts can never read them.
pub fn set_session(jar: CookieJar, session: &AuthSession, settings: CookieSettings) -> CookieJar {
    jar.add(build(ACCESS_COOKIE, session.access_token.clone(), session.access_expires_at, settings))
        .add(build(REFRESH_COOKIE, session.refresh_token.clone(), session.refresh_expires_at, settings))
}

pub fn clear_session(jar: CookieJar) -> CookieJar {
    jar.remove(Cookie::build(ACCESS_COOKIE).path("/")).remove(Cookie::build(REFRESH_COOKIE).path("/"))
}
