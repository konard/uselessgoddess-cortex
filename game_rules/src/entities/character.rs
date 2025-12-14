//! Character definitions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{EntityId, InventoryComponent, StatsComponent, StatusEffectComponent};
use crate::mechanics::StatusEffectType;

/// A full character definition with all relevant components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: EntityId,
    pub name: String,
    pub title: Option<String>,

    // Core components stored directly for frequent access
    pub stats: StatsComponent,
    pub inventory: InventoryComponent,
    pub status_effects: StatusEffectComponent,

    // Additional components in a flexible map
    #[serde(default)]
    pub extra_components: HashMap<String, serde_json::Value>,

    // Character-specific data
    pub backstory: Option<String>,
    pub personality_traits: Vec<String>,
    pub current_goal: Option<String>,
    /// Faction ID -> reputation score.
    pub faction_allegiances: HashMap<String, i32>,
}

impl Character {
    /// Create a new character with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: EntityId::new(),
            name: name.into(),
            title: None,
            stats: StatsComponent::default(),
            inventory: InventoryComponent::default(),
            status_effects: StatusEffectComponent::default(),
            extra_components: HashMap::new(),
            backstory: None,
            personality_traits: Vec::new(),
            current_goal: None,
            faction_allegiances: HashMap::new(),
        }
    }

    /// Check if the character is alive.
    pub fn is_alive(&self) -> bool {
        self.stats.current_hp > 0
    }

    /// Check if the character has a specific status effect.
    pub fn has_status(&self, effect: StatusEffectType) -> bool {
        self.status_effects
            .active_effects
            .iter()
            .any(|e| e.effect_type == effect)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_character() {
        let character = Character::new("Test Hero");
        assert_eq!(character.name, "Test Hero");
        assert!(character.is_alive());
        assert!(character.title.is_none());
    }

    #[test]
    fn test_character_death() {
        let mut character = Character::new("Doomed");
        character.stats.current_hp = 0;
        assert!(!character.is_alive());
    }
}
