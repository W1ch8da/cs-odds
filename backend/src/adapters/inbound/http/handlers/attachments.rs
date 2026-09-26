use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::Redirect,
};

use crate::{
    adapters::inbound::http::{
        dto::{StartUploadRequest, UploadSlotDto},
        extractors::{ApiJson, CurrentUser},
        state::HttpState,
    },
    application::{
        error::{AppError, AppResult},
        ports::inbound::StartUploadInput,
    },
    domain::attachment::AttachmentId,
};

/// Reserves a draft attachment and returns where to `PUT` the bytes.
pub async fn start_upload(
    State(state): State<HttpState>,
    CurrentUser(actor): CurrentUser,
    ApiJson(body): ApiJson<StartUploadRequest>,
) -> AppResult<(StatusCode, Json<UploadSlotDto>)> {
    let slot = state
        .attachments
        .start_upload(&actor, StartUploadInput { filename: body.filename, content_type: body.content_type, size: body.size })
        .await?;
    Ok((StatusCode::CREATED, Json(slot.into())))
}

/// Redirects to a short-lived download link after checking access.
pub async fn download(
    State(state): State<HttpState>,
    CurrentUser(actor): CurrentUser,
    Path(id): Path<String>,
) -> AppResult<Redirect> {
    let id = id.parse().map(AttachmentId).map_err(|_| AppError::NotFound)?;
    Ok(Redirect::to(&state.attachments.download_url(&actor, id).await?))
}
