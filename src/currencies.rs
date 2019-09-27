use failure::{format_err, Error};
use futures::TryStreamExt;
use hyper::Client;
use hyper_rustls::HttpsConnector;
use serde::{Deserialize, Serialize};

const ECB_DAILY: &str = "https://www.ecb.europa.eu/stats/eurofxref/eurofxref-daily.xml";
const ECB_HIST: &str = "https://www.ecb.europa.eu/stats/eurofxref/eurofxref-hist.xml";

#[derive(Debug, Deserialize)]
pub struct Envelope {
    #[serde(rename = "Cube", default)]
    cube: Cube,
}

#[derive(Debug, Deserialize, Default)]
pub struct Cube {
    #[serde(rename = "Cube", default)]
    dates: Vec<Date>,
}

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct Date {
    #[serde(rename = "time", default)]
    pub value: String,
    #[serde(rename = "Cube", default)]
    pub currencies: Vec<Currency>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Currency {
    pub currency: String,
    pub rate: f64,
}

pub async fn fetch_hist() -> Result<Vec<Date>, Error> {
    fetch(ECB_HIST).await
}

pub async fn fetch_daily() -> Result<Date, Error> {
    let mut dates = fetch(ECB_DAILY).await?;
    let dates = dates
        .pop()
        .ok_or_else(|| format_err!("Daily rates fetched from ECB are empty"))?;
    Ok(dates)
}

pub async fn fetch(url: &str) -> Result<Vec<Date>, Error> {
    let https = HttpsConnector::new();
    let client: Client<_, hyper::Body> = Client::builder().build(https);
    let res = client.get(url.parse::<hyper::Uri>()?).await?;
    let body = res.into_body().try_concat().await?;
    let envelope: Envelope = serde_xml_rs::from_reader(body.as_ref())
        .map_err(|err| format_err!("error parsing curencies from ECB {}", err))?;
    Ok(envelope.cube.dates)
}
