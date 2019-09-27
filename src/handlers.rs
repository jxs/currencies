use std::sync::Arc;

use askama::Template;

use crate::currencies::{fetch_daily, Currency, Date};
use crate::db::Db;

#[derive(Template)]
#[template(path = "index.html")]
struct CurrenciesTemplate<'a> {
    date: &'a str,
    currencies: &'a [Currency],
}

fn render(date: Date) -> Result<impl warp::Reply, warp::Rejection> {
    let rendered = CurrenciesTemplate {
        date: &date.value,
        currencies: date.currencies.as_slice(),
    }
    .render()
    .map_err(|e| warp::reject::custom(e))?;

    Ok(warp::reply::html(rendered))
}

pub async fn index(db: Arc<Db>) -> Result<impl warp::Reply, warp::Rejection> {
    let currencies = db
        .get_current_rates()
        .await
        .map_err(|e| warp::reject::custom(e))?;

    render(currencies)
}
