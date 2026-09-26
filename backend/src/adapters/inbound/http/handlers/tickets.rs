use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};

use crate::{
    adapters::inbound::http::{
        dto::{
            AddCommentRequest, CreateTicketRequest, ListTicketsQuery, PageDto, TicketDetailDto, TicketSummaryDto,
            UpdateTicketRequest, ViewCountsDto,
        },
        extractors::{ApiJson, CurrentUser},
        state::HttpState,
    },
    application::{
        error::{AppError, AppResult},
        ports::inbound::{AddCommentInput, AssigneeFilter, CreateTicketInput, ListTicketsInput, TicketSort, UpdateTicketInput},
    },
    domain::{
        attachment::AttachmentId,
        ticket::{Priority, Status, TicketNumber},
        user::UserId,
    },
};

/// Accepts `1042` or `TKT-1042`. Anything else is simply not found.
fn ticket_number(raw: &str) -> AppResult<TicketNumber> {
    let digits = raw.get(..4).filter(|p| p.eq_ignore_ascii_case("tkt-")).map_or(raw, |_| &raw[4..]);
    digits.parse().map(TicketNumber).map_err(|_| AppError::NotFound)
}

fn parse_opt<T: std::str::FromStr>(raw: Option<&str>) -> Result<Option<T>, T::Err> {
    raw.map(str::parse).transpose()
}

pub async fn list(
    State(state): State<HttpState>,
    CurrentUser(actor): CurrentUser,
    Query(q): Query<ListTicketsQuery>,
) -> AppResult<Json<PageDto<TicketSummaryDto>>> {
    let statuses = match q.status.as_deref().filter(|s| !s.is_empty()) {
        Some(list) => list.split(',').map(|s| s.trim().parse::<Status>()).collect::<Result<Vec<_>, _>>()?,
        None => Vec::new(),
    };
    let assignee = match q.assignee.as_deref() {
        None | Some("") => None,
        Some("me") => Some(AssigneeFilter::Me),
        Some("unassigned") => Some(AssigneeFilter::Unassigned),
        Some(id) => Some(AssigneeFilter::User(UserId(id.parse().map_err(|_| {
            AppError::Validation("assignee must be \"me\", \"unassigned\" or a user id.".into())
        })?))),
    };
    let sort = match q.sort.as_deref() {
        None | Some("priority") => TicketSort::Priority,
        Some("recent") => TicketSort::Recent,
        Some(_) => return Err(AppError::Validation("sort must be \"priority\" or \"recent\".".into())),
    };
    let page = state
        .tickets
        .list(
            &actor,
            ListTicketsInput {
                statuses,
                priority: parse_opt::<Priority>(q.priority.as_deref().filter(|p| !p.is_empty()))?,
                assignee,
                search: q.q,
                sort,
                page: q.page.unwrap_or(1),
                per_page: q.per_page.unwrap_or(0),
            },
        )
        .await?;
    Ok(Json(page.into()))
}

pub async fn counts(State(state): State<HttpState>, CurrentUser(actor): CurrentUser) -> AppResult<Json<ViewCountsDto>> {
    Ok(Json(state.tickets.view_counts(&actor).await?.into()))
}

pub async fn create(
    State(state): State<HttpState>,
    CurrentUser(actor): CurrentUser,
    ApiJson(body): ApiJson<CreateTicketRequest>,
) -> AppResult<(StatusCode, Json<TicketDetailDto>)> {
    let input = CreateTicketInput {
        subject: body.subject,
        description: body.description,
        priority: parse_opt(body.priority.as_deref())?,
        requester_email: body.requester_email,
        requester_name: body.requester_name,
        assignee_id: body.assignee_id.map(UserId),
        attachment_ids: body.attachment_ids.into_iter().map(AttachmentId).collect(),
    };
    let ticket = state.tickets.create(&actor, input).await?;
    Ok((StatusCode::CREATED, Json(ticket.into())))
}

pub async fn get(
    State(state): State<HttpState>,
    CurrentUser(actor): CurrentUser,
    Path(number): Path<String>,
) -> AppResult<Json<TicketDetailDto>> {
    Ok(Json(state.tickets.get(&actor, ticket_number(&number)?).await?.into()))
}

pub async fn update(
    State(state): State<HttpState>,
    CurrentUser(actor): CurrentUser,
    Path(number): Path<String>,
    ApiJson(body): ApiJson<UpdateTicketRequest>,
) -> AppResult<Json<TicketDetailDto>> {
    let input = UpdateTicketInput {
        status: parse_opt(body.status.as_deref())?,
        priority: parse_opt(body.priority.as_deref())?,
        assignee_id: body.assignee_id.map(|a| a.map(UserId)),
    };
    Ok(Json(state.tickets.update(&actor, ticket_number(&number)?, input).await?.into()))
}

pub async fn add_comment(
    State(state): State<HttpState>,
    CurrentUser(actor): CurrentUser,
    Path(number): Path<String>,
    ApiJson(body): ApiJson<AddCommentRequest>,
) -> AppResult<(StatusCode, Json<TicketDetailDto>)> {
    let input = AddCommentInput {
        body: body.body,
        internal: body.internal,
        status_after: parse_opt(body.status.as_deref())?,
        attachment_ids: body.attachment_ids.into_iter().map(AttachmentId).collect(),
    };
    let ticket = state.tickets.add_comment(&actor, ticket_number(&number)?, input).await?;
    Ok((StatusCode::CREATED, Json(ticket.into())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticket_number_formats() {
        assert_eq!(ticket_number("1042").unwrap(), TicketNumber(1042));
        assert_eq!(ticket_number("TKT-1042").unwrap(), TicketNumber(1042));
        assert!(matches!(ticket_number("abc"), Err(AppError::NotFound)));
    }
}
