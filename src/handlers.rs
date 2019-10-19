use std::cmp::Ordering;
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

// order currencies so that EUR comes first then gomes USD and then GBP
fn sort_currencies(currencies: &mut Vec<Currency>) {
    currencies.sort_by(
        |curr1, curr2| match (curr1.name.as_ref(), curr2.name.as_ref()) {
            ("EUR", _) => Ordering::Less,
            (_, "EUR") => Ordering::Greater,
            ("USD", "GBP") | ("GBP", "USD") => Ordering::Equal,
            ("USD", _) => Ordering::Less,
            (_, "USD") => Ordering::Greater,
            ("GBP", _) => Ordering::Less,
            (_, "GBP") => Ordering::Greater,
            _ => Ordering::Equal,
        },
    );
}

pub async fn index(db: Arc<Db>) -> Result<impl warp::Reply, warp::Rejection> {
    let mut date = db
        .get_current_rates()
        .await
        .map_err(|e| warp::reject::custom(e))?;

    sort_currencies(&mut date.currencies);
    let rendered = CurrenciesTemplate {
        date: &date.value,
        currencies: date.currencies.as_slice(),
    }
    .render()
    .map_err(|e| warp::reject::custom(e))?;

    Ok(warp::reply::html(rendered))
}

#[cfg(test)]
mod tests {
    use super::{sort_currencies, Currency};

    #[test]
    fn test_sort_currencies() {
        let mut currencies = Vec::new();
        currencies.push(Currency {
            name: "JPY".to_string(),
            rate: 0.0,
        });
        currencies.push(Currency {
            name: "RON".to_string(),
            rate: 0.0,
        });
        currencies.push(Currency {
            name: "USD".to_string(),
            rate: 0.0,
        });
        currencies.push(Currency {
            name: "CZK".to_string(),
            rate: 0.0,
        });
        currencies.push(Currency {
            name: "GBP".to_string(),
            rate: 0.0,
        });
        currencies.push(Currency {
            name: "CHF".to_string(),
            rate: 0.0,
        });
        currencies.push(Currency {
            name: "EUR".to_string(),
            rate: 0.0,
        });
        currencies.push(Currency {
            name: "RUB".to_string(),
            rate: 0.0,
        });
        sort_currencies(&mut currencies);
        assert_eq!(&currencies[0].name, "EUR");
        assert_eq!(&currencies[1].name, "USD");
        assert_eq!(&currencies[2].name, "GBP");
    }
}
