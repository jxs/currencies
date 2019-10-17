use std::fmt;

use serde::Serialize;
use warp::http::StatusCode;
use warp::{Rejection, Reply};

#[derive(Serialize)]
struct ErrorMessage {
    code: u16,
    msg: String,
}

// needed while https://github.com/seanmonstar/warp/issues/5 isn't solved
pub enum RecoverReply<A, B> {
    Json(A),
    Html(B),
}

impl<A, B> Reply for RecoverReply<A, B>
where
    A: Reply + Send,
    B: Reply + Send,
{
    fn into_response(self) -> warp::reply::Response {
        match self {
            RecoverReply::Json(r) => r.into_response(),
            RecoverReply::Html(r) => r.into_response(),
        }
    }
}

pub async fn recover(err: Rejection) -> Result<RecoverReply<impl Reply, impl Reply>, Rejection> {
    //api errors should be returned in json
    if let Some(ref err) = err.find_cause::<Reject>() {
        let error = match err {
            Reject::InvalidDateFormat(_, _)
            | Reject::PastDate(_)
            | Reject::InvalidSymbol
            | Reject::MissingDateBoundaries
            | Reject::InvalidDateRange
            | Reject::InvalidBase(_) => {
                log::trace!("api reject, {}", err);
                ErrorMessage {
                    code: StatusCode::BAD_REQUEST.as_u16(),
                    msg: err.to_string()
                }
            }
            Reject::DateNotFound(_) => {
                log::trace!("api reject, {}", err);
                ErrorMessage {
                    code: StatusCode::NOT_FOUND.as_u16(),
                    msg: StatusCode::NOT_FOUND
                        .canonical_reason()
                        .unwrap()
                        .to_string(),
                }
            }
            Reject::Unhandled(e) => {
                log::error!("api error, {}", e);
                ErrorMessage {
                    code: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                    msg: StatusCode::INTERNAL_SERVER_ERROR
                        .canonical_reason()
                        .unwrap()
                        .to_string()
                }
            }
        };

        return Ok(RecoverReply::Json(warp::reply::with_status(
            warp::reply::json(&error),
            StatusCode::from_u16(error.code).unwrap()
        )));
    }

    return Ok(RecoverReply::Html(StatusCode::INTERNAL_SERVER_ERROR));
}

#[derive(Debug)]
pub enum Reject {
    DateNotFound(String),
    PastDate(&'static str),
    InvalidDateRange,
    InvalidDateFormat(&'static str, String),
    InvalidBase(String),
    InvalidSymbol,
    MissingDateBoundaries,
    Unhandled(warp::reject::Cause),
}

impl fmt::Display for Reject {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Reject::PastDate(param) => f.write_str(&format!(
                "{} is invalid, there are no currency rates for dates older then 1999-01-04.",
                param
            )),
            Reject::DateNotFound(date) => {
                f.write_str(&format!("no curencies found for date {}", date))
            }
            Reject::InvalidDateFormat(param, date) => f.write_str(&format!(
                "{}: {} is in an invalid date format, date must be in the format %Y-%m-%d",
                param, date
            )),
            Reject::InvalidBase(base) => {
                f.write_str(&format!("{} is an invalid base currency", base))
            }
            Reject::MissingDateBoundaries => {
                f.write_str("both start_at and end_at parameters must be present")
            }
            Reject::InvalidSymbol => f.write_str("symbol list contains invalid symbols"),
            Reject::InvalidDateRange => f.write_str("start_at must be older than end_at"),
            Reject::Unhandled(err) => f.write_str(&format!("unhandled error, {}", err)),
        }
    }
}

impl std::error::Error for Reject {}
