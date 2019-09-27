use std::cmp::Ordering;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use chrono::naive::NaiveDate;
use failure::{format_err, Error, ResultExt};
use rocksdb::{Direction, IteratorMode, Options, DB};
use serde::{de::DeserializeOwned, Serialize};
use tokio_executor::blocking;

use crate::currencies::{Currency, Date};

pub async fn init() -> Result<Db, Error> {
    let path = Path::new("db");
    if path.exists() {
        log::info!("previous db file found, going to open it");
        Db::open(path)
    } else {
        Db::bootstrap_new(path).await
    }
}

pub struct Db {
    inner: Arc<rocksdb::DB>,
}

impl Db {
    pub async fn bootstrap_new<P: AsRef<Path>>(path: P) -> Result<Db, Error> {
        log::info!("no database found, going to bootstrap a new one");
        log::info!("dowloading ECB's currency values since 99");
        let dates = crate::currencies::fetch_hist().await.with_context(|e| {
            format!("could not fetch Historical reference rates from ECB {}", e)
        })?;

        log::info!("populating new db with currency values");
        let current_date = dates
            .first()
            .ok_or_else(|| format_err!("fetched Historical reference rates from ECB are empy"))?;

        let db = Db::open(path)?;

        db.put("current", &current_date.value).await?;
        for mut date in dates {
            let day = date.value;
            //insert EUR base
            date.currencies.push(Currency {
                currency: "EUR".to_string(),
                rate: 1.0,
            });
            db.put(&day, &date.currencies).await?;
        }
        db.run(|db| db.flush()).await?;

        Ok(db)
    }

    fn open<P: AsRef<Path>>(path: P) -> Result<Db, Error> {
        let mut options = Options::default();
        options.create_if_missing(true);
        options.set_comparator("dates", |first, second| {
            let first =
                std::str::from_utf8(first).expect("could not parse first db key in comparator");
            let second =
                std::str::from_utf8(second).expect("could not parse second db key in comparator");

            // we only want to sort date types, but they all should be followed
            match (NaiveDate::from_str(first), NaiveDate::from_str(second)) {
                (Ok(first), Ok(second)) => first.cmp(&second),
                (Err(_), Ok(_)) => Ordering::Greater,
                (Ok(_), Err(_)) => Ordering::Less,
                (Err(_), Err(_)) => Ordering::Equal,
            }
        });

        let db = Db {
            inner: Arc::new(rocksdb::DB::open(&options, path)?),
        };
        Ok(db)
    }

    pub async fn get_current_rates(&self) -> Result<Date, Error> {
        let current = self
            .get::<String>("current")
            .await?
            .ok_or_else(|| format_err!("could not find `current` key on the database"))?;

        let result = self.get::<Vec<Currency>>(&current).await?.ok_or_else(|| {
            format_err!("could not find `current` reference rates on the database")
        })?;

        Ok(Date {
            value: current,
            currencies: result,
        })
    }

    pub async fn get_day_rate(&self, day: &str) -> Result<Option<Date>, Error> {
        match self.get::<Vec<Currency>>(day).await? {
            Some(currencies) => Ok(Some(Date {
                value: day.to_string(),
                currencies: currencies,
            })),
            None => Ok(None),
        }
    }

    pub async fn get_range_rates(
        &self,
        start_at: NaiveDate,
        end_at: NaiveDate,
    ) -> Result<Vec<Date>, Error> {
        self.run(move |db| {
            let mut results = Vec::new();
            let iter = db.iterator(IteratorMode::From(
                end_at.to_string().as_bytes(),
                Direction::Reverse,
            ));
            for (key, value) in iter {
                let date =
                    std::str::from_utf8(&key).context("could not parse database  key as string")?;
                let date = NaiveDate::from_str(date)
                    .context("could not parse database key as NaiveDate")?;
                if date >= start_at {
                    let currencies = bincode::deserialize::<Vec<Currency>>(&value[..])
                        .with_context(|e| format!("could Deserialize database value"))?;
                    results.push(Date {
                        value: date.to_string(),
                        currencies: currencies,
                    });
                } else {
                    break;
                }
            }
            Ok(results)
        })
        .await
    }

    async fn put<T: Serialize>(&self, key: &str, value: &T) -> Result<(), Error> {
        let encoded: Vec<u8> = bincode::serialize(value)?;
        let key = key.to_string();
        let db = self.inner.clone();

        blocking::run(move || db.put(&key, encoded))
            .await
            .map_err(|e| e.into())
    }

    async fn get<T>(&self, key: &str) -> Result<Option<T>, Error>
    where
        T: DeserializeOwned + Send + 'static,
    {
        let key = key.to_string();

        self.run(move |db| {
            let blob = match db.get(&key)? {
                Some(blob) => blob,
                None => return Ok(None),
            };

            let t = bincode::deserialize::<T>(&blob[..])?;
            Ok(Some(t))
        })
        .await
    }

    pub async fn run<F, T>(&self, f: F) -> T
    where
        F: FnOnce(Arc<rocksdb::DB>) -> T + Send + 'static,
        T: Send + 'static,
    {
        let db = self.inner.clone();
        blocking::run(|| f(db)).await
    }
}
