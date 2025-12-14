//! Tag definitions - nodes in the knowledge graph.

use game_rules::{EntityId, LocationId};
use serde::{Deserialize, Serialize};

/// Tags are the nodes in our knowledge graph.
/// They represent concepts, entities, locations, or themes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tag {
    /// Reference to a specific entity (character, creature, item).
    Entity(EntityId),

    /// A location in the world.
    Location(LocationId),

    /// A concept or theme (e.g., "Magic", "Combat", "Love").
    Concept(String),

    /// A faction or organization.
    Faction(String),

    /// An event type (e.g., "Battle", "Meeting").
    EventType(String),

    /// A relationship type (e.g., "Friend", "Enemy").
    RelationType(String),

    /// Custom tag for extension.
    Custom(String),
}

impl Tag {
    /// Create a new entity tag.
    pub fn entity(id: EntityId) -> Self {
        Tag::Entity(id)
    }

    /// Create a new location tag.
    pub fn location(id: LocationId) -> Self {
        Tag::Location(id)
    }

    /// Create a new concept tag.
    pub fn concept(name: impl Into<String>) -> Self {
        Tag::Concept(name.into())
    }

    /// Create a new faction tag.
    pub fn faction(name: impl Into<String>) -> Self {
        Tag::Faction(name.into())
    }

    /// Create a new event type tag.
    pub fn event_type(name: impl Into<String>) -> Self {
        Tag::EventType(name.into())
    }

    /// Create a new relationship type tag.
    pub fn relation_type(name: impl Into<String>) -> Self {
        Tag::RelationType(name.into())
    }

    /// Create a custom tag.
    pub fn custom(name: impl Into<String>) -> Self {
        Tag::Custom(name.into())
    }

    /// Convert the tag to a string representation.
    pub fn as_string(&self) -> String {
        match self {
            Tag::Entity(id) => format!("entity:{}", id.0),
            Tag::Location(id) => format!("location:{}", id.0),
            Tag::Concept(s) => format!("concept:{}", s),
            Tag::Faction(s) => format!("faction:{}", s),
            Tag::EventType(s) => format!("event:{}", s),
            Tag::RelationType(s) => format!("relation:{}", s),
            Tag::Custom(s) => format!("custom:{}", s),
        }
    }

    /// Get the category/type of this tag.
    pub fn category(&self) -> &'static str {
        match self {
            Tag::Entity(_) => "entity",
            Tag::Location(_) => "location",
            Tag::Concept(_) => "concept",
            Tag::Faction(_) => "faction",
            Tag::EventType(_) => "event",
            Tag::RelationType(_) => "relation",
            Tag::Custom(_) => "custom",
        }
    }
}

impl std::fmt::Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_creation() {
        let concept = Tag::concept("Magic");
        assert!(matches!(concept, Tag::Concept(s) if s == "Magic"));

        let faction = Tag::faction("Knights");
        assert!(matches!(faction, Tag::Faction(s) if s == "Knights"));
    }

    #[test]
    fn test_tag_as_string() {
        let concept = Tag::concept("Combat");
        assert_eq!(concept.as_string(), "concept:Combat");

        let entity = Tag::entity(EntityId::nil());
        assert!(entity.as_string().starts_with("entity:"));
    }

    #[test]
    fn test_tag_equality() {
        let tag1 = Tag::concept("Magic");
        let tag2 = Tag::concept("Magic");
        let tag3 = Tag::concept("Combat");

        assert_eq!(tag1, tag2);
        assert_ne!(tag1, tag3);
    }

    #[test]
    fn test_tag_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(Tag::concept("Magic"));
        set.insert(Tag::concept("Magic")); // Duplicate

        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_tag_category() {
        assert_eq!(Tag::concept("Test").category(), "concept");
        assert_eq!(Tag::entity(EntityId::nil()).category(), "entity");
        assert_eq!(Tag::faction("Guild").category(), "faction");
    }
}
