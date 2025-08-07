//! # Simulation Decisions Crate (`sim_decisions`)
//!
//! This crate provides the framework for agent intelligence and behavior. It defines the
//! core interface through which all agents, regardless of their type (consumer, bank, firm),
//! make decisions and generate actions.
//!
//! ## Crate Structure and Purpose
//!
//! The central concept in this crate is the `DecisionModel` trait. This trait acts as a
//! universal contract for agent AI. The simulation engine iterates through all agents,
//! calling the `decide` method on their assigned `DecisionModel` to collect a list of
//! `SimAction`s to be executed in the current tick.
//!
//! This approach decouples the agent's state (its data, like a `Consumer` struct) from its
//! behavior (its logic, like a `BasicConsumerDecisionModel`), allowing for flexible and
//! interchangeable AI strategies.
//!
//! Key modules:
//! - **`decision_models.rs`**: Contains implementations of the `DecisionModel` trait, such as a
//!   prototype machine learning-based model (`MLDecisionModel`).
//!
//! ## Key Components
//!
//! - **`DecisionModel` (trait)**: The core abstraction for agent behavior. Its single required
//!   method, `decide`, takes the agent's state and the overall simulation state to produce a
//!   vector of `SimAction`s. This is the "brain" of an agent.
//!
//! - **`decide(...)` (method)**: The function that encapsulates an agent's logic. It assesses the
//!   current state and determines what the agent should do next, returning zero or more actions.
pub mod decision_models;
pub use decision_models::*;