# Currencies API
[![Build Status](https://github.com/jxs/currencies/workflows/Rust/badge.svg)](https://github.com/jxs/currencies/actions)

Currency rates API, greatly inspired by [exchangeratesapi](https://github.com/exchangeratesapi/exchangeratesapi), is a free service for current and historical foreign exchange rates [published by the European Central Bank](https://www.ecb.europa.eu/stats/policy_and_exchange_rates/euro_reference_exchange_rates/html/index.en.html).

## Usage

### Web usage:

Currencies offers a web interface with the current rates, available at ```GET /```
![alt text](https://raw.githubusercontent.com/jxs/currencies/master/screenshot.png)

### API
#### Lates & specific date rates
Get the latest foreign exchange rates.

```http
GET /api/v1/latest
```

Get historical rates for any day since 1999.

```http
GET /api/v1/2018-03-26
```

Rates are quoted against the Euro by default. Quote against a different currency by setting the base parameter in your request.

```http
GET /api/v1/latest?base=USD
```

Request specifi exchange rates by setting the symbols parameter.

```http
GET /api/v1/latest?symbols=USD,GBP
```

#### Rates history
Get historical rates for a time period.

```http
GET /api/v1/history?start_at=2018-01-01&end_at=2018-09-01
```

Limit results to specific exchange rates to save bandwidth with the symbols parameter.

```http
GET /api/v1/history?start_at=2018-01-01&end_at=2018-09-01&symbols=ILS,JPY
```

Quote the historical rates against a different currency.

```http
GET /api/v1/history?start_at=2018-01-01&end_at=2018-09-01&base=USD
```

#### Client side usage

The primary use case is client side. For instance, with [money.js](https://openexchangerates.github.io/money.js/) in the browser

```js
let demo = () => {
  let rate = fx(1).from("GBP").to("USD")
  alert("£1 = $" + rate.toFixed(4))
}

fetch('https://currencies.info.tm/api/v1/latest')
  .then((resp) => resp.json())
  .then((data) => fx.rates = data.rates)
  .then(demo)
```

## Deployment
deploy via Dockerfile to desired environment, define **PORT** and **DB_LOCATION** env vars for service port, and database file location respectively

#### Load in initial data & Scheduler
The scheduler will keep service's database up to date hourly with information from European Central bank. It will check current rates from ECB, and if database lacks any date between ECB's first currency rates and it's current, scheduler with download missing days

_The reference rates are usually updated around 16:00 CET on every working day, except on TARGET closing days. They are based on a regular daily concertation procedure between central banks across Europe, which normally takes place at 14:15 CET._

On initialization it will check the database. If it's empty all the historic rates will be downloaded and records created in the database.

## Contributing
Thanks for your interest in the project! All pull requests are welcome from developers of all skill levels. To get started, simply fork the master branch on GitHub to your personal account and then clone the fork into your development environment.

João Oliveira

## Credits
Madis Väin (madisvain on Github, Twitter), the original creator of the [Exchange Rates API framework](https://github.com/exchangeratesapi/exchangeratesapi).

## License
MIT
