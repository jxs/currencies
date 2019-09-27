mod currencies;

use warp::Filter;
use currencies::{Currency, Date, get_latest};
use askama::Template;

#[derive(Template)]
#[template(path = "index.html")]
struct CurrenciesTemplate<'a> {
    date: &'a str,
    currencies: &'a [Currency],
}

fn render(date: Date) -> Result<impl warp::Reply, warp::Rejection> {
    let rendered = CurrenciesTemplate {
        date: &date.time,
        currencies: date.currencies.as_slice()
    }
    .render()
    .unwrap();
    Ok(warp::reply::html(rendered))
}

#[tokio::main]
async fn main() {
    // Match `/:u32`...
    let routes = warp::path::end()
        // and_then create a `Future` that will simply wait N seconds...
        .and_then(|| async move {
            // delay(Instant::now() + Duration::from_secs(seconds)).await;
            let date = get_latest().await.unwrap();
            render(date)
        });

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
