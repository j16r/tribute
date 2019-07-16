# Tribute

Tribute is a Rust client for various crypto exchanges, which can export
transactions and create reports for the purpose of filing taxes.

## Configuration

Tribute takes its configuration from the file "config.toml" which is in the
[TOML format](https://github.com/toml-lang/toml).

An example is shown below:

    exchanges = [
      { Coinbase = { key = "<coinbase-key>", secret = "<coinbase-secret>" } },
      { CoinbasePro = { key = "<coinbase-pro-key>", secret = "<coinbase-pro-secret>", passphrase = "<coinbase-passphrase>" } },
    ]

## Exports

Tribute can export all transactions for either Coinbase or Coinbase Pro
exchanges, and outputs in CSV format.

## Reports

Given an exported CSV, tribute can output a "report". The report summarizes all
short term sells and includes cost basis and gain as required by [IRS Form
8949](http://www.irs.gov/Form8949).
