//! # Trading Domain Crate
//!
//! This crate provides the logic for agent interactions with the simulated markets.
//! It handles the mechanics of placing orders and the financial settlement of executed trades.
//!
//! ## Crate Structure and Purpose
//!
//! The core of this crate is the `TradingDomain` struct, which acts as the interface
//! between agents and the market exchange.
//!
//! - **`domain.rs`**: Implements the `TradingDomain` struct. This service has two main roles:
//!   1.  **Executing Trading Actions**: It handles `TradingAction`s like `PostBid` and `PostAsk`.
//!       When an agent decides to trade, this domain validates the action (e.g., does the seller
//!       have the asset to sell?) and then creates a `PlaceOrderInBook` market effect. The actual
//!       matching of bids and asks is handled by the `Exchange` in `sim_types`.
//!   2.  **Settling Trades**: After the `Exchange` matches orders and creates `Trade` records, the
//!       `TradingDomain`'s `settle_financial_trade` method is called. This method is responsible
//!       for creating the `StateEffect`s that represent the financial outcome of the trade:
//!       transferring the asset from the seller to the buyer and transferring payment from the
//!       buyer to the seller.
//!
//! This crate does *not* contain a `behavior.rs` module because the *decision* to trade
//! is made within each agent's own domain (e.g., a bank's `BasicBankDecisionModel` decides
//! to trade bonds). This crate only provides the *mechanism* for that trade to occur.
//!
//! ## Key Components
//!
//! - **`TradingDomain`**: The service for posting orders and settling completed trades.
//! - **`TradingAction`**: The actions for posting bids and asks (defined in `sim_actions`).
//! - **`Trade`**: A data structure from `sim_types` representing a matched trade to be settled.
pub mod domain;
pub use domain::*;