mod api;
mod currencies;
mod db;
mod errors;
mod handlers;

use db::Db;
use std::env;
use std::sync::Arc;
use std::time::{Duration, Instant};

use exitfailure::ExitDisplay;
use failure::{format_err, Error};
use warp::Filter;

#[tokio::main]
async fn main() -> Result<(), ExitDisplay<Error>> {
    env_logger::init();
    let port = env::var("PORT")
        .unwrap_or("3030".to_string())
        .parse()
        .map_err(|e| format_err!("could not parse port as valid number, {}", e))?;

    let db_location = std::env::var("DB_LOCATION").unwrap_or_else(|_| "db".to_string());
    let db = Db::init(&db_location).await?;
    let db_filter = Arc::new(db.clone());

    // launch updater daemon
    tokio::spawn(async move {
        let mut interval = tokio::timer::Interval::new(Instant::now(), Duration::from_secs(360));
        while let Some(_) = interval.next().await {
            db.update().await.expect("error updating database!");
        }
    });

    let api = api::routes(db_filter.clone());

    let ui = warp::path::end()
        .and(warp::get2())
        .map(move || db_filter.clone())
        .and_then(handlers::index);

    let routes = api.or(ui).recover(errors::recover);

    warp::serve(routes).run(([0, 0, 0, 0], port)).await;
    Ok(())
}
