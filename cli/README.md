# Breez SDK - *Ark* CLI

A simple cli tool that sends commands to the sdk. It is intended to demonstrate the usage and investigate issues that are hard to debug on mobile platforms.

## Run
Currently regtest and signet is supported.
Easiest way is to start the cli with signet configuration

```bash
cargo run -- --network signet --data-dir <data directory>
```

## Commands

To get a full list of commands run `-h` or `<command> -h` to get more information about a command.