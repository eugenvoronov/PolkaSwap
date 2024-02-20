# PolkaSwap: Decentralized Exchange on Polkadot

![PolkaSwap Logo](https://s2.coinmarketcap.com/static/img/coins/64x64/6636.png)

## Overview

PolkaSwap is a decentralized exchange (DEX) built on the Polkadot ecosystem, designed to provide users with a seamless and secure platform for trading various assets. Inspired by Uniswap V2, PolkaSwap brings the power of decentralized finance (DeFi) to the Polkadot network, allowing users to swap assets in a trustless and permissionless manner.

This pallet allows users to create liquidity pools, add or remove liquidity, and swap assets seamlessly within the Polkadot network.

## Features

- **Decentralized swapping:** Swap various assets directly from your wallet without the need for a centralized intermediary.

- **Liquidity pools:** Contribute to liquidity pools and earn fees by providing liquidity to the decentralized exchange.

- **Polkadot ecosystem integration:** Built on the Polkadot network, ensuring security, scalability, and interoperability.

## TODO
I actively working on improving the Polkadot DEX Pallet. The following tasks are currently in progress:

- Price Oracle: I'm developing a robust price oracle to provide accurate asset pair prices within the pools.

- Benchmarking: Work is underway to create comprehensive benchmarks for the pallet to ensure optimal performance.

## How to use 
```rust
let dot = 1u32;
let usdt = 2u32;

let liquidity_dot = 200_000u128 * UNIT; // 1 dot for 5 usdt price
let liquidity_usdt = 1_000_000u128 * UNIT;
let amount_a_min = 1u128;
let amount_b_min = 1u128;

assert_ok!(
  Dex::add_liquidity(
    RuntimeOrigin::signed(sender),
    dot.clone(),
    usdt.clone(),
    liquidity_dot,
    liquidity_usdt,
    amount_a_min,
    amount_b_min,
    sender,
  )
);

let amount_in = 10u128 * UNIT; // usdt

assert_ok!(
  Dex::swap_exact_asset_for_asset(
    RuntimeOrigin::signed(sender),
    usdt,
    dot,
    amount_in,
    1,
  )
);
```

## Installation and setup

Please first check the latest information on getting starting with Substrate dependencies required to build this project [here](https://docs.substrate.io/main-docs/install/).

Clone the repository:

```bash
git clone https://github.com/pba-private/2024-hk-assignment-4-frame-eugenvoronov.git
```

### Development Testing

To test while developing, without a full build (thus reduce time to results):

```sh
cargo t -p pallet-dex
```

### Build

Build the node without launching it, with `release` optimizations:

```sh
cargo b -r
```

### Run

Build and launch the node, with `release` optimizations:

```sh
cargo r -r -- --dev
```

### CLI Docs

Once the project has been built, the following command can be used to explore all CLI arguments and subcommands:

```sh
./target/release/node-template -h
```

## Functions

### 1. Create pool
This function allows the origin (an account) to create a new liquidity pool by specifying two assets (asset_a and asset_b) and the desired liquidity pool token (lp_asset). It returns a DispatchResult indicating whether the operation was successful.

#### Function signature:

```rust
pub fn create_pool(
    origin: OriginFor<T>,
    asset_a: AssetIdOf<T>,
    asset_b: AssetIdOf<T>,
    lp_asset: AssetIdOf<T>,
) -> DispatchResult;
```

### 2. Add liquidity
This function allows the origin (an account) to add liquidity to an existing pool by providing assets asset_a and asset_b. It calculates the optimal amount of assets to contribute based on the current reserves and desired amounts, then mints new liquidity tokens and transfers them to the specified account (mint_to).

#### Function signature:

```rust
pub fn add_liquidity(
    origin: OriginFor<T>,
    asset_a: AssetIdOf<T>,
    asset_b: AssetIdOf<T>,
    amount_a_desired: BalanceOf<T>,
    amount_b_desired: BalanceOf<T>,
    amount_a_min: BalanceOf<T>,
    amount_b_min: BalanceOf<T>,
    mint_to: T::AccountId,
) -> DispatchResult;
```

### 3. Remove liquidity
This function allows the origin (an account) to remove liquidity from an existing pool by burning specified liquidity pool tokens (lp_token_burn). It calculates the amount of assets asset_a and asset_b to be received in return for burned LP tokens, ensuring that the received amounts meet the specified minimum requirements.

#### Function signature:

```rust
pub fn remove_liquidity(
    origin: OriginFor<T>,
    asset_a: AssetIdOf<T>,
    asset_b: AssetIdOf<T>,
    lp_token_burn: BalanceOf<T>,
    amount_a_min_receive: BalanceOf<T>,
    amount_b_min_receive: BalanceOf<T>,
) -> DispatchResult;
```

### 4. Swap exact asset for asset
This function allows the origin (an account) to execute a swap where the sender provides a specific amount of input asset (exact_in) to receive at least a minimum amount of output asset (asset_out). It performs the swap and calculates the resulting amount based on the current reserves.

#### Function signature:

```rust
pub fn swap_exact_asset_for_asset(
    origin: OriginFor<T>,
    exact_in: AssetIdOf<T>,
    asset_out: AssetIdOf<T>,
    amount_in: BalanceOf<T>,
    amount_out_min: BalanceOf<T>,
) -> DispatchResult;
```

### 5. Swap asset for exact asset
This function allows the origin (an account) to execute a swap where the sender provides a maximum amount of input asset (asset_in) to receive a specific amount of output asset (exact_out). It performs the swap and calculates the resulting amount based on the current reserves.

#### Function signature:

```rust
pub fn swap_asset_for_exact_asset(
    origin: OriginFor<T>,
    asset_in: AssetIdOf<T>,
    exact_out: AssetIdOf<T>,
    amount_out: BalanceOf<T>,
    amount_in_max: BalanceOf<T>,
) -> DispatchResult;
```

## License
This PolkaSwap is licensed under the MIT License.