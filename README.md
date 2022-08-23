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

### Manual Transactions

If your exchange is not supported by Tribute, or you get sent tokens via some
other means, you can add transactions to Tribute's configuration.

    [[transactions]]
    id = "776cf8bb-a6e3-4b43-aa98-bb338e11e0be"
    market = "ETH-USD"
    token = "ETH"
    amount = 6572
    rate = 0.26
    usd_rate = 0.26
    usd_amount = 1692
    created_at = 2018-01-17

## Exports

Tribute can export all transactions for either Coinbase or Coinbase Pro
exchanges, and outputs in CSV format. When you export, all transactions from
your configured exchanges and all manual transactions are ordered by date and
emitted.

## Reports

Given an exported CSV, Tribute can output a "report". The report summarizes all
short term sells and includes cost basis and gain as required by [IRS Form
8949](http://www.irs.gov/Form8949).
