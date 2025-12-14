//! # Narrative Core (The Cortex)
//!
//! The "brain" of the emergent narrative system. This crate interfaces with
//! `game_rules`, manages knowledge through an associative graph, and assembles
//! context for LLM-driven narrative generation.
//!
//! ## Core Components
//!
//! - **knowledge_base**: Long-term memory using an associative knowledge graph
//! - **context_assembler**: Builds context using spreading activation algorithm
//! - **events**: Game event types for core<->rules communication
//!
//! ## Design Philosophy
//!
//! - **State-Driven**: All narrative decisions are based on current world state and accumulated memory
//! - **Event-Driven**: The core reacts to game events, not controlling the game loop
//! - **Extensible**: New fact types, tags, and behaviors can be added without modifying core logic

pub mod context_assembler;
pub mod events;
pub mod knowledge_base;

pub use context_assembler::*;
pub use events::*;
pub use knowledge_base::*;
