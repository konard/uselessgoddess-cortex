//! Entity definitions for the game world.

mod character;
mod components;

pub use character::*;
pub use components::*;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for all entities in the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(pub Uuid);

impl EntityId {
    /// Create a new random entity ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create an entity ID from a specific UUID.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Create a nil/empty entity ID (useful for defaults).
    pub fn nil() -> Self {
        Self(Uuid::nil())
    }
}

impl Default for EntityId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Types of entities in the game world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityType {
    Character,
    Creature,
    Item,
    Location,
}
