use std::sync::Arc;

use askama::Template;

use crate::currencies::Currency;
use crate::db::Db;

#[derive(Template)]
#[template(path = "index.html")]
struct CurrenciesTemplate<'a> {
    date: &'a str,
    currencies: &'a [Currency],
}

pub async fn index(db: Arc<Db>) -> Result<impl warp::Reply, warp::Rejection> {
    let date = db
        .get_current_rates()
        .await
        .map_err(|e| warp::reject::custom(e))?;

    let rendered = CurrenciesTemplate {
        date: &date.value,
        currencies: date.currencies.as_slice(),
    }
    .render()
    .map_err(|e| warp::reject::custom(e))?;

    Ok(warp::reply::html(rendered))
}
