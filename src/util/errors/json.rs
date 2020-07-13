use std::fmt;

use super::AppError;
use crate::util::{json_response, AppResponse};

use chrono::NaiveDateTime;
use conduit::{header, StatusCode};

/// Generates a response with the provided status and description as JSON
fn json_error(detail: &str, status: StatusCode) -> AppResponse {
    #[derive(Serialize)]
    struct StringError<'a> {
        detail: &'a str,
    }
    #[derive(Serialize)]
    struct Bad<'a> {
        errors: Vec<StringError<'a>>,
    }

    let mut response = json_response(&Bad {
        errors: vec![StringError { detail }],
    });
    *response.status_mut() = status;
    response
}

// The following structs are emtpy and do not provide a custom message to the user

#[derive(Debug)]
pub(crate) struct NotFound;

// This struct has this helper impl for use as `NotFound.into()`
impl From<NotFound> for AppResponse {
    fn from(_: NotFound) -> AppResponse {
        json_error("Not Found", StatusCode::NOT_FOUND)
    }
}

impl AppError for NotFound {
    fn response(&self) -> Option<AppResponse> {
        Some(Self.into())
    }
}

impl fmt::Display for NotFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "Not Found".fmt(f)
    }
}

#[derive(Debug)]
pub(super) struct Forbidden;
#[derive(Debug)]
pub(crate) struct ReadOnlyMode;

impl AppError for Forbidden {
    fn response(&self) -> Option<AppResponse> {
        let detail = "must be logged in to perform that action";
        Some(json_error(detail, StatusCode::FORBIDDEN))
    }
}

impl fmt::Display for Forbidden {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "must be logged in to perform that action".fmt(f)
    }
}

impl AppError for ReadOnlyMode {
    fn response(&self) -> Option<AppResponse> {
        let detail = "Crates.io is currently in read-only mode for maintenance. \
                      Please try again later.";
        Some(json_error(detail, StatusCode::SERVICE_UNAVAILABLE))
    }
}

impl fmt::Display for ReadOnlyMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "Tried to write in read only mode".fmt(f)
    }
}

// The following structs wrap owned data and provide a custom message to the user

#[derive(Debug)]
pub(super) struct Ok(pub(super) String);
#[derive(Debug)]
pub(super) struct BadRequest(pub(super) String);
#[derive(Debug)]
pub(super) struct ServerError(pub(super) String);
#[derive(Debug)]
pub(crate) struct TooManyRequests {
    pub retry_after: NaiveDateTime,
}

impl AppError for Ok {
    fn response(&self) -> Option<AppResponse> {
        Some(json_error(&self.0, StatusCode::OK))
    }
}

impl fmt::Display for Ok {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AppError for BadRequest {
    fn response(&self) -> Option<AppResponse> {
        Some(json_error(&self.0, StatusCode::BAD_REQUEST))
    }
}

impl fmt::Display for BadRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AppError for ServerError {
    fn response(&self) -> Option<AppResponse> {
        Some(json_error(&self.0, StatusCode::INTERNAL_SERVER_ERROR))
    }
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AppError for TooManyRequests {
    fn response(&self) -> Option<AppResponse> {
        use std::convert::TryInto;

        const HTTP_DATE_FORMAT: &str = "%a, %d %b %Y %H:%M:%S GMT";
        let retry_after = self.retry_after.format(HTTP_DATE_FORMAT);

        let detail = format!(
            "You have published too many crates in a \
             short period of time. Please try again after {} or email \
             help@crates.io to have your limit increased.",
            retry_after
        );
        let mut response = json_error(&detail, StatusCode::TOO_MANY_REQUESTS);
        response.headers_mut().insert(
            header::RETRY_AFTER,
            retry_after
                .to_string()
                .try_into()
                .expect("HTTP_DATE_FORMAT contains invalid char"),
        );
        Some(response)
    }
}

impl fmt::Display for TooManyRequests {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "Too many requests".fmt(f)
    }
}
