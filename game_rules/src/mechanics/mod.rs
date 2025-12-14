//! Game mechanics: damage types, status effects, skills, etc.

use serde::{Deserialize, Serialize};

/// All possible damage types in the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DamageType {
    // Physical
    Slashing,
    Piercing,
    Bludgeoning,

    // Elemental
    Fire,
    Cold,
    Lightning,
    Acid,

    // Magical
    Radiant,
    Necrotic,
    Force,
    Psychic,

    // Special
    Poison,
    /// Bypasses all resistances.
    True,
}

/// Status effects that can be applied to entities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StatusEffectType {
    // Damage over time
    Bleeding,
    Poisoned,
    Burning,

    // Control
    Stunned,
    Paralyzed,
    Frightened,
    Charmed,

    // Debuffs
    Weakened,
    Slowed,
    Blinded,
    Deafened,

    // Buffs
    Blessed,
    Hasted,
    Invisible,
    Protected,

    // Special
    Unconscious,
    Dead,
}

/// Equipment slots for characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EquipmentSlot {
    Head,
    Chest,
    Hands,
    Legs,
    Feet,
    MainHand,
    OffHand,
    Neck,
    Ring1,
    Ring2,
}

/// Combat range categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombatRange {
    Melee,
    /// 5-15 feet.
    Close,
    /// 15-60 feet.
    Medium,
    /// 60-120 feet.
    Long,
    /// 120+ feet.
    Extreme,
}

/// Resistance types for damage calculation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResistanceType {
    /// 2x damage.
    Vulnerable,
    /// 1x damage.
    Normal,
    /// 0.5x damage.
    Resistant,
    /// 0x damage.
    Immune,
}

impl ResistanceType {
    /// Get the damage multiplier for this resistance type.
    pub fn multiplier(&self) -> f32 {
        match self {
            ResistanceType::Vulnerable => 2.0,
            ResistanceType::Normal => 1.0,
            ResistanceType::Resistant => 0.5,
            ResistanceType::Immune => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resistance_multipliers() {
        assert_eq!(ResistanceType::Vulnerable.multiplier(), 2.0);
        assert_eq!(ResistanceType::Normal.multiplier(), 1.0);
        assert_eq!(ResistanceType::Resistant.multiplier(), 0.5);
        assert_eq!(ResistanceType::Immune.multiplier(), 0.0);
    }
}
