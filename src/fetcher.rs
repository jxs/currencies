use crate::errors::Error;
use chrono::NaiveDate;
use hyper::Client;
use hyper_rustls::HttpsConnector;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

const ECB_DAILY: &str = "https://www.ecb.europa.eu/stats/eurofxref/eurofxref-daily.xml";
const ECB_HIST: &str = "https://www.ecb.europa.eu/stats/eurofxref/eurofxref-hist.xml";
const ECB_HIST_LAST_90: &str = "https://www.ecb.europa.eu/stats/eurofxref/eurofxref-hist-90d.xml";

#[derive(Debug, Deserialize)]
pub struct Envelope {
    #[serde(rename = "Cube", default)]
    pub cube: Cube,
}

#[derive(Debug, Deserialize, Default)]
pub struct Cube {
    #[serde(rename = "Cube", default)]
    pub dates: Vec<Date>,
}

#[derive(Clone, Debug, Deserialize, Default, PartialEq, Serialize)]
pub struct Date {
    #[serde(rename = "time", default)]
    pub value: String,
    #[serde(rename = "Cube", default)]
    pub currencies: Vec<Currency>,
}

impl Date {
    pub fn value_as_date(&self) -> Result<NaiveDate, Error> {
        NaiveDate::from_str(&self.value).map_err(|_| Error::DateParseError(self.value.clone()))
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Currency {
    #[serde(rename = "currency", default)]
    pub name: String,
    pub rate: f64,
}

pub async fn fetch_last90() -> Result<Vec<Date>, Error> {
    fetch(ECB_HIST_LAST_90).await
}

pub async fn fetch_hist() -> Result<Vec<Date>, Error> {
    fetch(ECB_HIST).await
}

pub async fn fetch_daily() -> Result<Date, Error> {
    let mut dates = fetch(ECB_DAILY).await?;
    let dates = dates
        .pop()
        .ok_or_else(|| Error::FetcherError("Daily rates are empty".into()))?;
    Ok(dates)
}

pub async fn fetch(url: &str) -> Result<Vec<Date>, Error> {
    let https = HttpsConnector::new();
    let client: Client<_, hyper::Body> = Client::builder().build(https);
    let res =
        client
            .get(url.parse::<hyper::Uri>().map_err(|err| {
                Error::FetcherError(format!("could not parse url: {}, {}", url, err))
            })?)
            .await
            .map_err(|err| Error::FetcherError(err.to_string()))?;
    let body = hyper::body::to_bytes(res.into_body())
        .await
        .map_err(|err| Error::FetcherError(err.to_string()))?;
    let envelope: Envelope = serde_xml_rs::from_reader(body.as_ref())
        .map_err(|err| Error::FetcherError(err.to_string()))?;
    Ok(envelope.cube.dates)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fetch() {
        let current = fetch_daily().await.unwrap();
        current.value_as_date().unwrap();
    }

    #[test]
    fn value_as_date() {
        let date = Date {
            value: "1999-01-04".to_string(),
            currencies: Vec::new(),
        };
        let ddate = date.value_as_date().unwrap();
        assert_eq!("1999-01-04", &ddate.to_string());
    }
}
