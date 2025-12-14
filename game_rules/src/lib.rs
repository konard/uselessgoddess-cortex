//! # Game Rules
//!
//! The "World Bible" crate - contains all game rules, mechanics, and entity definitions.
//! This crate is the single source of truth for game state and does not contain any AI logic.

pub mod entities;
pub mod mechanics;
pub mod world_state;

pub use entities::*;
pub use mechanics::*;
pub use world_state::*;
