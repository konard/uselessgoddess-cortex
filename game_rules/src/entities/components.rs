//! Component definitions for entities.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::EntityId;
use crate::mechanics::{EquipmentSlot, StatusEffectType};

/// Stats component for characters and creatures.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsComponent {
    pub strength: i32,
    pub dexterity: i32,
    pub constitution: i32,
    pub intelligence: i32,
    pub wisdom: i32,
    pub charisma: i32,
    pub current_hp: i32,
    pub max_hp: i32,
    pub current_mana: i32,
    pub max_mana: i32,
}

impl Default for StatsComponent {
    fn default() -> Self {
        Self {
            strength: 10,
            dexterity: 10,
            constitution: 10,
            intelligence: 10,
            wisdom: 10,
            charisma: 10,
            current_hp: 10,
            max_hp: 10,
            current_mana: 0,
            max_mana: 0,
        }
    }
}

/// Stat types for modifier calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatType {
    Strength,
    Dexterity,
    Constitution,
    Intelligence,
    Wisdom,
    Charisma,
}

impl StatsComponent {
    /// Calculate modifier for a given stat (D&D style: (stat - 10) / 2).
    pub fn modifier(&self, stat: StatType) -> i32 {
        let value = match stat {
            StatType::Strength => self.strength,
            StatType::Dexterity => self.dexterity,
            StatType::Constitution => self.constitution,
            StatType::Intelligence => self.intelligence,
            StatType::Wisdom => self.wisdom,
            StatType::Charisma => self.charisma,
        };
        (value - 10) / 2
    }
}

/// Inventory component for entities that can hold items.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InventoryComponent {
    pub items: Vec<ItemStack>,
    pub capacity: usize,
    pub equipped: HashMap<EquipmentSlot, EntityId>,
}

/// A stack of items in inventory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemStack {
    pub item_id: EntityId,
    pub quantity: u32,
}

/// Status effects currently active on an entity.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StatusEffectComponent {
    pub active_effects: Vec<ActiveStatusEffect>,
}

/// An active status effect with duration and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveStatusEffect {
    pub effect_type: StatusEffectType,
    /// None = permanent effect.
    pub remaining_duration: Option<u32>,
    pub stacks: u32,
    pub source: Option<EntityId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_modifier() {
        let stats = StatsComponent {
            strength: 18,
            dexterity: 14,
            constitution: 12,
            intelligence: 8,
            wisdom: 10,
            charisma: 16,
            ..Default::default()
        };

        assert_eq!(stats.modifier(StatType::Strength), 4);
        assert_eq!(stats.modifier(StatType::Dexterity), 2);
        assert_eq!(stats.modifier(StatType::Constitution), 1);
        assert_eq!(stats.modifier(StatType::Intelligence), -1);
        assert_eq!(stats.modifier(StatType::Wisdom), 0);
        assert_eq!(stats.modifier(StatType::Charisma), 3);
    }

    #[test]
    fn test_default_stats() {
        let stats = StatsComponent::default();
        assert_eq!(stats.strength, 10);
        assert_eq!(stats.modifier(StatType::Strength), 0);
    }
}
