//! Fact definitions - data entries in the knowledge graph.

use game_rules::{EntityId, QuestId, WorldTime};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

use super::Tag;

/// Unique identifier for facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FactId(pub Uuid);

impl FactId {
    /// Create a new random fact ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for FactId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for FactId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A fact is a piece of knowledge stored in the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    pub id: FactId,

    /// Human-readable content of the fact.
    pub content: String,

    /// Type of fact for filtering and processing.
    pub fact_type: FactType,

    /// Tags this fact is associated with.
    pub tags: HashSet<Tag>,

    /// When this fact was added (game time).
    pub timestamp: WorldTime,

    /// Importance score (0.0 - 1.0) for prioritization.
    pub importance: f32,

    /// Whether this fact is known to the player.
    pub known_to_player: bool,

    /// Whether this fact has been "revealed" in narrative.
    pub revealed: bool,

    /// Optional expiration (for temporary facts).
    pub expires_at: Option<WorldTime>,

    /// Source of this fact.
    pub source: FactSource,
}

impl Fact {
    /// Create a new fact with the given content.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            id: FactId::new(),
            content: content.into(),
            fact_type: FactType::Generic,
            tags: HashSet::new(),
            timestamp: WorldTime::default(),
            importance: 0.5,
            known_to_player: true,
            revealed: false,
            expires_at: None,
            source: FactSource::Initial,
        }
    }

    /// Set the fact type.
    pub fn with_type(mut self, fact_type: FactType) -> Self {
        self.fact_type = fact_type;
        self
    }

    /// Add a tag to this fact.
    pub fn with_tag(mut self, tag: Tag) -> Self {
        self.tags.insert(tag);
        self
    }

    /// Add multiple tags to this fact.
    pub fn with_tags(mut self, tags: impl IntoIterator<Item = Tag>) -> Self {
        self.tags.extend(tags);
        self
    }

    /// Set the importance score.
    pub fn with_importance(mut self, importance: f32) -> Self {
        self.importance = importance.clamp(0.0, 1.0);
        self
    }

    /// Set whether the fact is known to the player.
    pub fn with_known_to_player(mut self, known: bool) -> Self {
        self.known_to_player = known;
        self
    }

    /// Set the fact source.
    pub fn with_source(mut self, source: FactSource) -> Self {
        self.source = source;
        self
    }

    /// Set the timestamp.
    pub fn with_timestamp(mut self, timestamp: WorldTime) -> Self {
        self.timestamp = timestamp;
        self
    }

    /// Check if this fact has a specific tag.
    pub fn has_tag(&self, tag: &Tag) -> bool {
        self.tags.contains(tag)
    }

    /// Check if this fact involves a specific entity.
    pub fn involves_entity(&self, entity_id: EntityId) -> bool {
        self.tags.contains(&Tag::Entity(entity_id))
    }

    /// Mark this fact as revealed.
    pub fn reveal(&mut self) {
        self.revealed = true;
        self.known_to_player = true;
    }
}

/// Types of facts in the knowledge base.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FactType {
    /// Relationship between two entities.
    Relationship {
        entity_a: EntityId,
        entity_b: EntityId,
        relationship: RelationshipType,
        /// Sentiment from -1.0 (hostile) to 1.0 (friendly).
        sentiment: f32,
    },

    /// A historical event.
    Event {
        description: String,
        participants: Vec<EntityId>,
        location: Option<game_rules::LocationId>,
    },

    /// A secret that could be revealed.
    Secret {
        holder: EntityId,
        severity: SecretSeverity,
    },

    /// Character trait or personality note.
    Trait {
        entity: EntityId,
        trait_name: String,
    },

    /// World lore.
    Lore { category: String },

    /// Quest-related information.
    Quest { quest_id: QuestId },

    /// Generic/custom fact.
    Generic,
}

/// Relationship types between entities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipType {
    Family,
    Friend,
    Enemy,
    Romantic,
    Professional,
    Rival,
    Mentor,
    Acquaintance,
}

/// Severity levels for secrets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretSeverity {
    /// Embarrassing but harmless.
    Minor,
    /// Damaging to reputation.
    Moderate,
    /// Life-changing consequences.
    Major,
    /// Could result in death or worse.
    Critical,
}

/// Sources of facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FactSource {
    /// Part of initial game setup.
    Initial,
    /// Created by AI during play.
    LLMGenerated,
    /// Result of player choice.
    PlayerAction,
    /// Triggered by game mechanics.
    WorldEvent,
    /// Revealed through conversation.
    DialogueRevealed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fact_creation() {
        let fact = Fact::new("The hero defeated the dragon");
        assert_eq!(fact.content, "The hero defeated the dragon");
        assert_eq!(fact.importance, 0.5);
        assert!(fact.known_to_player);
        assert!(!fact.revealed);
    }

    #[test]
    fn test_fact_builder() {
        let fact = Fact::new("Secret alliance")
            .with_importance(0.9)
            .with_known_to_player(false)
            .with_tag(Tag::concept("Politics"))
            .with_tag(Tag::faction("Kingdom"))
            .with_source(FactSource::Initial);

        assert_eq!(fact.importance, 0.9);
        assert!(!fact.known_to_player);
        assert_eq!(fact.tags.len(), 2);
        assert!(fact.has_tag(&Tag::concept("Politics")));
    }

    #[test]
    fn test_fact_importance_clamping() {
        let fact_high = Fact::new("Test").with_importance(1.5);
        assert_eq!(fact_high.importance, 1.0);

        let fact_low = Fact::new("Test").with_importance(-0.5);
        assert_eq!(fact_low.importance, 0.0);
    }

    #[test]
    fn test_fact_reveal() {
        let mut fact = Fact::new("Hidden truth").with_known_to_player(false);
        assert!(!fact.known_to_player);
        assert!(!fact.revealed);

        fact.reveal();
        assert!(fact.known_to_player);
        assert!(fact.revealed);
    }

    #[test]
    fn test_relationship_fact() {
        let entity_a = EntityId::new();
        let entity_b = EntityId::new();

        let fact = Fact::new("They are childhood friends")
            .with_type(FactType::Relationship {
                entity_a,
                entity_b,
                relationship: RelationshipType::Friend,
                sentiment: 0.8,
            })
            .with_tag(Tag::entity(entity_a))
            .with_tag(Tag::entity(entity_b));

        assert!(fact.involves_entity(entity_a));
        assert!(fact.involves_entity(entity_b));
    }
}
