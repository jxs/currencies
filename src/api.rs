use crate::db::Db;
use crate::error::Error;
use crate::fetcher::Date;

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::json;
use warp::{Filter, Rejection, Reply};

pub fn routes(db: Arc<Db>) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    // /api/v1 endpoint
    let apiv1 = warp::path("api").and(warp::path("v1"));
    let db = warp::any().map(move || db.clone());

    let latest_head = apiv1
        .and(warp::path("latest"))
        .and(warp::path::end())
        .and(warp::head())
        .map(|| warp::reply::json(&()));

    let latest_get = apiv1
        .and(warp::path("latest"))
        .and(warp::path::end())
        .and(warp::get())
        .and(warp::query::<Params>())
        .and(db.clone())
        .and_then(latest_handler);

    let history_get = apiv1
        .and(warp::path("history"))
        .and(warp::path::end())
        .and(warp::get())
        .and(warp::query::<Params>())
        .and(db.clone())
        .and_then(history_handler);

    let day_get = apiv1
        .and(warp::path::param::<NaiveDate>())
        .and(warp::path::end())
        .and(warp::get())
        .and(warp::query::<Params>())
        .and(db)
        .and_then(day_handler);

    latest_head.or(history_get).or(latest_get).or(day_get)
}

#[derive(Default, Debug, Deserialize)]
struct Params {
    start_at: Option<String>,
    end_at: Option<String>,
    base: Option<String>,
    symbols: Option<String>,
}

async fn latest_handler(params: Params, db: Arc<Db>) -> Result<impl Reply, Rejection> {
    let currencies = db.get_current_rates().await?;

    Ok(try_reply(vec![currencies], params)?)
}

async fn day_handler(
    date: NaiveDate,
    params: Params,
    db: Arc<Db>,
) -> Result<impl Reply, Rejection> {
    if date < NaiveDate::from_ymd(1999, 1, 4) {
        return Err(Error::PastDate("date").into());
    }

    let currencies = db
        .get_day_rates(&date.to_string())
        .await?
        .ok_or_else(move || Error::DateNotFound(date.to_string()))?;

    Ok(try_reply(vec![currencies], params)?)
}

async fn history_handler(params: Params, db: Arc<Db>) -> Result<impl Reply, Rejection> {
    let (start_at, end_at) = match params {
        Params {
            start_at: Some(ref start_at),
            end_at: Some(ref end_at),
            ..
        } => {
            let start_at = NaiveDate::from_str(start_at)
                .map_err(move |_| Error::InvalidDateFormat("start_at", start_at.to_string()))?;

            let end_at = NaiveDate::from_str(end_at)
                .map_err(move |_| Error::InvalidDateFormat("end_at", end_at.to_string()))?;

            if start_at < NaiveDate::from_ymd(1999, 1, 4) {
                return Err(Error::PastDate("start_at").into());
            }

            if end_at < start_at {
                return Err(Error::InvalidDateRange.into());
            }

            (start_at, end_at)
        }
        _ => return Err(Error::MissingDateBoundaries.into()),
    };

    let currencies = db.get_range_rates(start_at, end_at).await?;

    Ok(try_reply(currencies, params)?)
}

fn try_reply(dates: Vec<Date>, params: Params) -> Result<impl Reply, Rejection> {
    let first = dates.get(0).ok_or_else(|| Error::EmpyDataset)?;

    let symbols = match params.symbols {
        Some(symbols_params) => {
            let symbols = symbols_params
                .split(',')
                .map(String::from)
                .collect::<Vec<String>>();
            if !symbols
                .iter()
                .all(|s| first.currencies.iter().any(|c| &c.name == s))
            {
                return Err(Error::InvalidSymbol.into());
            }
            symbols
        }
        None => Vec::new(),
    };

    let mut rates = HashMap::new();

    for date in dates.into_iter() {
        let mut currencies = HashMap::new();

        let base_rate = match params.base {
            None => 1.0,
            Some(ref base) => date
                .currencies
                .iter()
                .find(|b| &b.name == base)
                .map(|b| b.rate)
                .ok_or_else(|| Error::InvalidBase(base.to_string()))?,
        };

        for currency in date.currencies.into_iter() {
            if symbols.is_empty() || symbols.contains(&currency.name) {
                currencies.insert(currency.name, currency.rate / base_rate);
            }
        }

        rates.insert(date.value, currencies);
    }

    let base = params.base.unwrap_or_else(|| "EUR".to_string());
    let response = if rates.len() < 2 {
        // safe to call unwrap as we have already checked dates have at least one element
        let (date, rates) = rates.into_iter().next().unwrap();
        json! ({
            "rates": rates,
            "base": base,
            "date": date
        })
    } else {
        json! ({
            "rates": rates,
            "base": base,
            "start_at": params.start_at,
            "end_at": params.end_at,
        })
    };
    Ok(warp::reply::json(&response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetcher::Envelope;
    use std::fs::File;

    #[test]
    fn try_reply_returns_err_on_empty_dates() {
        let dates = Vec::new();
        let params = Params::default();
        let reply = try_reply(dates, params);
        assert!(reply.is_err());
    }

    #[tokio::test]
    async fn try_reply_multiple_days() {
        let file = File::open("seed_rates.xml").unwrap();
        let envelope: Envelope = serde_xml_rs::from_reader(&file).unwrap();
        let dates = envelope.cube.dates;
        let mut rates = HashMap::new();
        for date in dates.clone() {
            let mut currencies = HashMap::new();
            for currency in date.currencies {
                currencies.insert(currency.name, currency.rate);
            }
            rates.insert(date.value, currencies);
        }

        let mut params = Params::default();
        params.start_at = Some("2019-07-22".to_string());
        params.end_at = Some("2019-10-18".to_string());
        let response = try_reply(dates, params).unwrap().into_response();
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.as_ref().to_vec()).unwrap();
        let json = json!({
            "rates": rates,
            "base": "EUR",
            "start_at": "2019-07-22",
            "end_at": "2019-10-18"
        });
        assert_eq!(json.to_string(), body_str);
    }

    #[tokio::test]
    async fn try_reply_single_day() {
        let file = File::open("seed_rates.xml").unwrap();
        let envelope: Envelope = serde_xml_rs::from_reader(&file).unwrap();
        let mut dates = envelope.cube.dates;
        let mut rates = HashMap::new();
        for date in dates.clone() {
            let mut currencies = HashMap::new();
            for currency in date.currencies {
                currencies.insert(currency.name, currency.rate);
            }
            rates.insert(date.value, currencies);
        }

        let params = Params::default();
        let response = try_reply(vec![dates.pop().unwrap()], params)
            .unwrap()
            .into_response();
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.as_ref().to_vec()).unwrap();
        let day_rate = rates.remove("2019-07-22").unwrap();
        let json = json!({
            "rates": day_rate,
            "base": "EUR",
            "date": "2019-07-22"
        });
        assert_eq!(json.to_string(), body_str);
    }

    #[tokio::test]
    async fn try_reply_symbols_single_day() {
        let file = File::open("seed_rates.xml").unwrap();
        let envelope: Envelope = serde_xml_rs::from_reader(&file).unwrap();
        let mut dates = envelope.cube.dates;
        let mut rates = HashMap::new();
        for date in dates.clone() {
            let mut currencies = HashMap::new();
            for currency in date.currencies {
                if &currency.name == "USD" || currency.name == "JPY" {
                    currencies.insert(currency.name, currency.rate);
                }
            }
            rates.insert(date.value, currencies);
        }

        let mut params = Params::default();
        params.symbols = Some("USD,JPY".to_string());
        let response = try_reply(vec![dates.pop().unwrap()], params)
            .unwrap()
            .into_response();
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.as_ref().to_vec()).unwrap();
        let day_rate = rates.remove("2019-07-22").unwrap();
        let json = json!({
            "rates": day_rate,
            "base": "EUR",
            "date": "2019-07-22"
        });
        assert_eq!(json.to_string(), body_str);
    }

    #[tokio::test]
    async fn try_reply_symbols_multiple_days() {
        let file = File::open("seed_rates.xml").unwrap();
        let envelope: Envelope = serde_xml_rs::from_reader(&file).unwrap();
        let dates = envelope.cube.dates;
        let mut rates = HashMap::new();
        for date in dates.clone() {
            let mut currencies = HashMap::new();
            for currency in date.currencies {
                if &currency.name == "USD" || currency.name == "JPY" {
                    currencies.insert(currency.name, currency.rate);
                }
            }
            rates.insert(date.value, currencies);
        }

        let mut params = Params::default();
        params.start_at = Some("2019-07-22".to_string());
        params.end_at = Some("2019-10-18".to_string());
        params.symbols = Some("USD,JPY".to_string());
        let response = try_reply(dates, params).unwrap().into_response();
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.as_ref().to_vec()).unwrap();
        let json = json!({
            "rates": rates,
            "base": "EUR",
            "start_at": "2019-07-22",
            "end_at": "2019-10-18"
        });
        assert_eq!(json.to_string(), body_str);
    }

    #[tokio::test]
    async fn try_reply_different_base_single_day() {
        let file = File::open("seed_rates.xml").unwrap();
        let envelope: Envelope = serde_xml_rs::from_reader(&file).unwrap();
        let mut dates = envelope.cube.dates;
        let mut rates = HashMap::new();
        for date in dates.clone() {
            let mut currencies = HashMap::new();
            for currency in &date.currencies {
                let rate = date
                    .currencies
                    .iter()
                    .find(|b| &b.name == "GBP")
                    .map(|b| b.rate)
                    .unwrap();

                currencies.insert(currency.name.to_string(), currency.rate / rate);
            }
            rates.insert(date.value, currencies);
        }

        let mut params = Params::default();
        params.base = Some("GBP".to_string());
        let response = try_reply(vec![dates.pop().unwrap()], params)
            .unwrap()
            .into_response();
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.as_ref().to_vec()).unwrap();
        let day_rate = rates.remove("2019-07-22").unwrap();
        let json = json!({
            "rates": day_rate,
            "base": "GBP",
            "date": "2019-07-22"
        });
        assert_eq!(json.to_string(), body_str);
    }

    #[tokio::test]
    async fn try_reply_different_base_multiple_days() {
        let file = File::open("seed_rates.xml").unwrap();
        let envelope: Envelope = serde_xml_rs::from_reader(&file).unwrap();
        let dates = envelope.cube.dates;
        let mut rates = HashMap::new();
        for date in dates.clone() {
            let mut currencies = HashMap::new();
            for currency in &date.currencies {
                let rate = date
                    .currencies
                    .iter()
                    .find(|b| &b.name == "GBP")
                    .map(|b| b.rate)
                    .unwrap();

                currencies.insert(currency.name.to_string(), currency.rate / rate);
            }
            rates.insert(date.value, currencies);
        }

        let mut params = Params::default();
        params.base = Some("GBP".to_string());
        params.start_at = Some("2019-07-22".to_string());
        params.end_at = Some("2019-10-18".to_string());
        let response = try_reply(dates, params).unwrap().into_response();
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.as_ref().to_vec()).unwrap();
        let json = json!({
            "rates": rates,
            "base": "GBP",
            "start_at": "2019-07-22",
            "end_at": "2019-10-18"
        });
        assert_eq!(json.to_string(), body_str);
    }
}
