## Foundry

This repo contains a Foundry Solidity project and a Rust workspace under `crates/`. Run Solidity tests with `forge test` and the Rust crate with `cargo test -p stem`.

**Foundry is a blazing fast, portable and modular toolkit for Ethereum application development written in Rust.**

Foundry consists of:

- **Forge**: Ethereum testing framework (like Truffle, Hardhat and DappTools).
- **Cast**: Swiss army knife for interacting with EVM smart contracts, sending transactions and getting chain data.
- **Anvil**: Local Ethereum node, akin to Ganache, Hardhat Network.
- **Chisel**: Fast, utilitarian, and verbose solidity REPL.

## Documentation

https://book.getfoundry.sh/

## Usage

### Build

```shell
$ forge build
```

### Test

```shell
$ forge test
```

### Format

```shell
$ forge fmt
```

### Gas Snapshots

```shell
$ forge snapshot
```

### Anvil

```shell
$ anvil
```

### Deploy

Deploy the Stem contract (e.g. to local Anvil). The script prints the deployed address; use it with the stem examples.

```shell
$ anvil
$ forge script script/Deploy.s.sol:Deploy --rpc-url http://127.0.0.1:8545 --broadcast --private-key <your_private_key>
# Stem deployed at: 0x...  <- use this as --contract below
```

**Membrane (ocap API):** The membrane is the capability API on top of the Stem finalizer/adopted-epoch stream. Processes may persist across epochs, but privileged capabilities do not—authority is bound to the current adopted epoch. After a staleEpoch, clients must re-auth (e.g. `Membrane.login`) to obtain a new session.

**Stem examples (Rust):**

- **stem_indexer** — observed HeadUpdated events (no confirmations):
  `cargo run -p stem --example stem_indexer -- --rpc-url http://127.0.0.1:8545 --contract 0x...`

- **finalizer** — finalized events only (confirmation-depth + canonical cross-check); prints one-line JSON per event:
  `cargo run -p stem --example finalizer -- --ws-url ws://127.0.0.1:8545 --http-url http://127.0.0.1:8545 --contract 0x... [--depth 6]`

### Cast

```shell
$ cast <subcommand>
```

### Help

```shell
$ forge --help
$ anvil --help
$ cast --help
```
