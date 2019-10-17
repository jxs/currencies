use crate::currencies::Date;
use crate::db::Db;

use std::collections::HashMap;
use std::sync::Arc;
use std::str::FromStr;

use crate::errors::Reject;
use chrono::NaiveDate;
use anyhow::anyhow;
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
        .and(warp::get2())
        .and(warp::query::<Params>())
        .and(db.clone())
        .and_then(latest_handler);

    let history_get = apiv1
        .and(warp::path("history"))
        .and(warp::path::end())
        .and(warp::get2())
        .and(warp::query::<Params>())
        .and(db.clone())
        .and_then(history_handler);

    let day_get = apiv1
        .and(warp::path::param::<NaiveDate>())
        .and(warp::path::end())
        .and(warp::get2())
        .and(warp::query::<Params>())
        .and(db.clone())
        .and_then(day_handler);

    latest_head.or(history_get).or(latest_get).or(day_get)
}

#[derive(Debug, Deserialize)]
struct Params {
    start_at: Option<String>,
    end_at: Option<String>,
    base: Option<String>,
    symbols: Option<String>,
}

async fn latest_handler(params: Params, db: Arc<Db>) -> Result<impl Reply, Rejection> {
    let currencies = db
        .get_current_rates()
        .await
        .map_err(|e| warp::reject::custom(Reject::Unhandled(e.into())))?;
    Ok(try_reply(vec![currencies], params)?)
}

async fn day_handler(
    date: NaiveDate,
    params: Params,
    db: Arc<Db>,
) -> Result<impl Reply, Rejection> {
    if date < NaiveDate::from_ymd(1999, 1, 4) {
        return Err(warp::reject::custom(Reject::PastDate("date")));
    }

    let currencies = db
        .get_day_rates(&date.to_string())
        .await
        .map_err(|e| warp::reject::custom(Reject::Unhandled(e.into())))?
        .ok_or_else(move || warp::reject::custom(Reject::DateNotFound(date.to_string())))?;

    Ok(try_reply(vec![currencies], params)?)
}

async fn history_handler(params: Params, db: Arc<Db>) -> Result<impl Reply, Rejection> {
    let (start_at, end_at) = match params {
        Params {
            start_at: Some(ref start_at),
            end_at: Some(ref end_at),
            ..
        } => {
            let start_at = NaiveDate::from_str(start_at).map_err(move |_| {
                warp::reject::custom(Reject::InvalidDateFormat("start_at", start_at.to_string()))
            })?;

            let end_at = NaiveDate::from_str(end_at).map_err(move |_| {
                warp::reject::custom(Reject::InvalidDateFormat("end_at", end_at.to_string()))
            })?;

            if start_at < NaiveDate::from_ymd(1999, 1, 4) {
                return Err(warp::reject::custom(Reject::PastDate("start_at")));
            }

            if end_at < start_at {
                return Err(warp::reject::custom(Reject::InvalidDateRange));
            }

            (start_at, end_at)
        }
        _ => return Err(warp::reject::custom(Reject::MissingDateBoundaries)),
    };

    let currencies = db
        .get_range_rates(start_at, end_at)
        .await
        .map_err(|e| warp::reject::custom(Reject::Unhandled(e.into())))?;
    Ok(try_reply(currencies, params)?)
}

fn try_reply(dates: Vec<Date>, params: Params) -> Result<impl Reply, Rejection> {
    let first = dates.get(0).ok_or_else(|| {
        warp::reject::custom(Reject::Unhandled(
            anyhow!("empty currency dataset, should have at least 1 element").into(),
        ))
    })?;

    let (base, base_rate) = match params.base {
        None => ("EUR".to_string(), 1.0),
        Some(base) => first
            .currencies
            .iter()
            .find(|b| b.name == base)
            .map(|b| (b.name.clone(), b.rate))
            .ok_or_else(|| warp::reject::custom(Reject::InvalidBase(base)))?,
    };

    let symbols = match params.symbols {
        Some(symbols_params) => {
            let symbols = symbols_params
                .split(",")
                .map(String::from)
                .collect::<Vec<String>>();
            if !symbols
                .iter()
                .all(|s| first.currencies.iter().any(|c| &c.name == s))
            {
                return Err(warp::reject::custom(Reject::InvalidSymbol));
            }
            symbols
        }
        None => Vec::new(),
    };

    let mut rates = HashMap::new();

    for date in dates.into_iter() {
        let mut currencies = HashMap::new();

        for currency in date.currencies.into_iter() {
            if symbols.is_empty() || symbols.contains(&currency.name) {
                currencies.insert(currency.name, currency.rate / base_rate);
            }
        }

        rates.insert(date.value, currencies);
    }

    let response = if rates.len() < 2 {
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
