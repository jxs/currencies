mod api;
mod db;
mod errors;
mod fetcher;
mod handlers;

use std::env;
use std::sync::Arc;
use std::time::Duration;

use crate::errors::Error;
use exitfailure::ExitDisplay;
use futures::StreamExt;
use warp::Filter;

#[tokio::main]
async fn main() -> Result<(), ExitDisplay<Error>> {
    env_logger::init();
    let port = env::var("PORT").unwrap_or_else(|_| "3030".to_string());
    let port = port.parse().map_err(|_| Error::InvalidPort(port))?;

    let db_location = std::env::var("DB_LOCATION").unwrap_or_else(|_| "db".to_string());
    let db = db::init(&db_location).await?;
    let db_filter = Arc::new(db.clone());

    // launch updater daemon
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(360));
        while let Some(_) = interval.next().await {
            db::update(&db).await.expect("error updating database!");
        }
    });

    let api = api::routes(db_filter.clone());

    let ui = warp::path::end()
        .and(warp::get())
        .map(move || db_filter.clone())
        .and_then(handlers::index);

    let routes = api.or(ui).recover(errors::recover);

    warp::serve(routes).run(([0, 0, 0, 0], port)).await;
    Ok(())
}
