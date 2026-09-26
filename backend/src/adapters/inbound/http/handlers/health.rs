use axum::{Json, extract::State};

use crate::{
    adapters::inbound::http::HttpState,
    application::{error::AppResult, ports::inbound::HealthReport},
};

pub async fn health(State(state): State<HttpState>) -> AppResult<Json<HealthReport>> {
    Ok(Json(state.health.check().await?))
}
