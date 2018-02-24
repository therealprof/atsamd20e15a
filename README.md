# atsamd20e15a

A rust crate for the ATSAMD20 MCU including special support for the Snowflake (https://github.com/LuckyResistor/SnowFlakeProject)

## Quick "How to get going"

Make sure you have an `arm-none-eabi` toolchain installed.

Setup a Rust toolchain for Cortex-M processors:

```
$ rustup install nightly
$ rustup component add rust-src
$ rustup override set nightly
$ cargo install xargo
```

After this you can build the examples with:

```
$ xargo build --examples --release
```

and pick up the executables from
**target/thumbv6m-none-eabi/release/examples/**.

I've also added basic `openocd_program.sh` script to program your
MCU but this is meant to be used with a CMSIS-DAP debugger and may
not work with your debugger without modifying the **atsamd20.cfg**
OpenOCD configuration.

It is also possible to get binaries with debugging information by
leaving out the *--release* option but they may not fit into flash.

You may also have to modify **memory.x** in case your MCU has more
(or less) flash than the 32kB I assumed.
