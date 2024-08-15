# Kapacitor Multi-Indicator Stream UDF

## Overview

This project implements a User-Defined Function (UDF) for Kapacitor, providing streaming processing capabilities for multiple financial indicators. Currently, it supports Exponential Moving Average (EMA) and Simple Moving Average (SMA) calculations and can be easily extended for other indicators.

## Features

- Streaming processing of time series data
- Support for multiple indicators (EMA and SMA)
- Unix socket communication with Kapacitor
- Asynchronous processing using async-std
- Configurable options for each indicator

## Requirements

- Rust 1.70 or later
- Kapacitor 1.6 or later

## Installation

1. Clone the repository:

   ```bash

   git clone <https://github.com/suitable-name/kapacitor-udf-indicator-stream-rs>
   cd kapacitor-multi-indicator-stream-udf

   ```

2. Build / Install the project:

   ```bash

   cargo build --release

   ```

   or

   ```bash

   sudo cargo install --path . --root /usr/local

   ```

## Usage

1. Start the UDF server:

   ```bash

   ./target/release/kapacitor-multi-indicator-stream-udf

   ```

   By default, the server listens on `/tmp/indicator-stream.sock`. You can specify a different socket path using the `-s` or `--socket` option:

   ```bash

   ./target/release/kapacitor-multi-indicator-stream-udf -s /path/to/custom/socket.sock

   ```

2. Configure Kapacitor to use this UDF. Add the following to your Kapacitor configuration file:

   ```toml
   [udfs]
     [udfs.functions]
       [udfs.functions.indicator_stream]
         socket = "/tmp/kapacitor-multi-indicator-stream-udf"
         timeout = "10s"
   ```

3. Use the UDF in your TICKscripts:

   ```bash
   stream
       |from()
           .measurement('ticker_data')
       @indicator_stream()
           .type('EMA')
           .field('last_price')
           .as('last_price_ema')
           .period(14)
           .ticker_field('ticker')
       |influxDBOut()
           .database('stocks')
           .retentionPolicy('autogen')
           .measurement('Stocks_EMA_stream')
           .tag('kapacitor', 'true')
   ```

## Configuration Options

- `type`: The type of indicator to calculate (`EMA` or `SMA`)
- `period`: The period for the indicator calculation
- `field`: The field name in the incoming data to use for calculations
- `as`: The field name to use for the calculated indicator value in the output
- `ticker_field`: The field name containing the ticker or symbol for the data point

## Development

To run with debug logging:

```bash
RUST_LOG=debug ./target/release/kapacitor-multi-indicator-stream-udf
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
