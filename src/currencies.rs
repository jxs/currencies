use failure::{format_err, Error};
use serde::Deserialize;
use hyper::Client;
use futures::TryStreamExt;
use hyper_rustls::HttpsConnector;

const ECB_LATEST_URL: &str = "https://www.ecb.europa.eu/stats/eurofxref/eurofxref-daily.xml";

#[derive(Debug, Deserialize)]
pub struct Envelope {
    #[serde(rename = "Cube", default)]
    cube: Cube
}

#[derive(Debug, Deserialize, Default)]
pub struct Cube {
    #[serde(rename = "Cube", default)]
    dates: Vec<Date>
}

#[derive(Debug, Deserialize, Default)]
pub struct Date {
    pub time: String,
    #[serde(rename = "Cube", default)]
    pub currencies: Vec<Currency>
}

#[derive(Debug, Deserialize)]
pub struct Currency {
    pub currency: String,
    pub rate: f64,
}

pub async fn get_latest() -> Result<Date, Error> {
    let https = HttpsConnector::new();
    let client: Client<_, hyper::Body> = Client::builder().build(https);
    let res = client.get(ECB_LATEST_URL.parse::<hyper::Uri>()?).await?;
    let body = res.into_body().try_concat().await?;
    let mut envelope: Envelope  = serde_xml_rs::from_reader(body.as_ref())
        .map_err(|err| format_err!("error parsing curencies from ECB {}", err))?;
    Ok(envelope.cube.dates.pop().unwrap())
}