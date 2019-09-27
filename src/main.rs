mod api;
mod currencies;
mod db;
mod handlers;

use exitfailure::ExitDisplay;
use failure::Error;
use serde::Serialize;
use std::sync::Arc;
use warp::http::StatusCode;
use warp::{Filter, Rejection, Reply};

#[tokio::main]
async fn main() -> Result<(), ExitDisplay<Error>> {
    env_logger::init();
    let db = Arc::new(db::init().await?);

    let api = api::routes(db.clone());

    let ui = warp::path::end()
        .and(warp::get2())
        .map(move || db.clone())
        .and_then(handlers::index);

    let routes = api.or(ui).recover(recover);

    warp::serve(routes).run(([0, 0, 0, 0], 3030)).await;
    Ok(())
}

#[derive(Serialize)]
struct ErrorMessage {
    code: u16,
    msg: String,
}

async fn recover(err: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(ref err) = err.find_cause::<crate::api::Reject>() {
        let code = match err {
            api::Reject::MethodNotAllowed => StatusCode::METHOD_NOT_ALLOWED,
            api::Reject::InvalidDateFormat(_, _)
            | api::Reject::PastDate(_)
            | api::Reject::InvalidSymbol
            | api::Reject::MissingDateBoundaries
            | api::Reject::InvalidDateRange
            | api::Reject::InvalidBase(_) => StatusCode::BAD_REQUEST,
            api::Reject::DateNotFound(_) => StatusCode::NOT_FOUND,
        };
        let msg = err.to_string();
        let json = warp::reply::json(&ErrorMessage {
            code: code.as_u16(),
            msg,
        });
        return Ok(warp::reply::with_status(json, code));
    }

    Err(err)
}
