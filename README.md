Cosmos Hodler Snapshot
======================

Take a snapshot of token stakers on a Cosmos-SDK chain. Purpose-built for Juno
but probably works for other chains in the Cosmos ecosystem.

Usage
-----

This program takes a number of flags and commands. Specifying `--grpc` is always
required as that is how the program reads on-chain data. To get a snapshot of
delegators staking JUNO with validators that are not jailed run the following:

```
cosmos-hodler-snapshot --grpc=<gRPC-URI> native-stakers
```

These results are written to a CSV file: `./juno_stakers.csv` where amounts are
denominated using the native precision. In juno's case these are ujuno
(“micro-juno”) where 1000000ujuno is equal to 1 JUNO token.

NOTE: To snapshot the chain at a certain height [spin up a Juno
node](https://docs.junonetwork.io/validators/joining-mainnet) and halt it at the
desired snapshot height by setting `halt-height=<HEIGHT>` in the node's config.
This way a consistent snapshot is taken at a definitive height and no
inconsistencies will appear, as would be the case with taking a snapshot against
a live node that is actively consuming blocks.

Building
--------

Install Rust using `rustup` or however you like. Then just do `cargo build`;
nothing fancy here.

For an environment with `nix` installed one can run `nix-shell` to bootstrap a
dev environment without the need to fumble with `rustup`.

License
-------

MIT License (available under /LICENSE)
