//! Knowledge Graph - the core data structure for associative memory.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use super::{Fact, FactId, FactType, Tag};
use game_rules::EntityId;

/// Association between two tags with a weight.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Association {
    pub target: Tag,
    /// Weight from 0.0 to 1.0 indicating association strength.
    pub weight: f32,
    pub association_type: AssociationType,
}

/// Types of associations between tags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssociationType {
    /// Explicitly defined association.
    Direct,
    /// Both tags appear on same facts.
    CoOccurrence,
    /// Conceptually related.
    Semantic,
    /// Related in time.
    Temporal,
}

/// The main knowledge graph structure.
///
/// The graph stores facts (data) and associations (relationships between concepts).
/// It provides efficient lookups by tag, entity, and fact ID.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KnowledgeGraph {
    /// All facts stored by ID.
    facts: HashMap<FactId, Fact>,

    /// Index: Tag -> Facts containing this tag.
    tag_to_facts: HashMap<Tag, HashSet<FactId>>,

    /// Associations between tags (adjacency list).
    associations: HashMap<Tag, Vec<Association>>,

    /// Reverse index for efficient entity lookups.
    fact_by_entity: HashMap<EntityId, HashSet<FactId>>,
}

impl KnowledgeGraph {
    /// Create a new empty knowledge graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new fact to the knowledge base.
    ///
    /// Returns the fact ID for reference.
    pub fn add_fact(&mut self, fact: Fact) -> FactId {
        let id = fact.id;

        // Index by tags
        for tag in &fact.tags {
            self.tag_to_facts
                .entry(tag.clone())
                .or_default()
                .insert(id);
        }

        // Index by entity references
        match &fact.fact_type {
            FactType::Relationship {
                entity_a,
                entity_b,
                ..
            } => {
                self.fact_by_entity.entry(*entity_a).or_default().insert(id);
                self.fact_by_entity.entry(*entity_b).or_default().insert(id);
            }
            FactType::Secret { holder, .. } => {
                self.fact_by_entity.entry(*holder).or_default().insert(id);
            }
            FactType::Trait { entity, .. } => {
                self.fact_by_entity.entry(*entity).or_default().insert(id);
            }
            _ => {}
        }

        // Also index entity tags
        for tag in &fact.tags {
            if let Tag::Entity(entity_id) = tag {
                self.fact_by_entity
                    .entry(*entity_id)
                    .or_default()
                    .insert(id);
            }
        }

        self.facts.insert(id, fact);
        id
    }

    /// Remove a fact from the knowledge base.
    pub fn remove_fact(&mut self, id: FactId) -> Option<Fact> {
        if let Some(fact) = self.facts.remove(&id) {
            // Remove from tag index
            for tag in &fact.tags {
                if let Some(facts) = self.tag_to_facts.get_mut(tag) {
                    facts.remove(&id);
                }
            }

            // Remove from entity index
            for (_, facts) in self.fact_by_entity.iter_mut() {
                facts.remove(&id);
            }

            Some(fact)
        } else {
            None
        }
    }

    /// Get all facts associated with a tag.
    pub fn facts_by_tag(&self, tag: &Tag) -> Vec<&Fact> {
        self.tag_to_facts
            .get(tag)
            .map(|ids| ids.iter().filter_map(|id| self.facts.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get all facts associated with an entity.
    pub fn facts_by_entity(&self, entity_id: EntityId) -> Vec<&Fact> {
        self.fact_by_entity
            .get(&entity_id)
            .map(|ids| ids.iter().filter_map(|id| self.facts.get(id)).collect())
            .unwrap_or_default()
    }

    /// Add or update an association between tags.
    pub fn add_association(
        &mut self,
        from: Tag,
        to: Tag,
        weight: f32,
        assoc_type: AssociationType,
    ) {
        let associations = self.associations.entry(from).or_default();

        // Update existing or add new
        if let Some(existing) = associations.iter_mut().find(|a| a.target == to) {
            // Average weights for updates
            existing.weight = (existing.weight + weight) / 2.0;
        } else {
            associations.push(Association {
                target: to,
                weight: weight.clamp(0.0, 1.0),
                association_type: assoc_type,
            });
        }
    }

    /// Add bidirectional association between tags.
    pub fn add_bidirectional_association(
        &mut self,
        tag_a: Tag,
        tag_b: Tag,
        weight: f32,
        assoc_type: AssociationType,
    ) {
        self.add_association(tag_a.clone(), tag_b.clone(), weight, assoc_type);
        self.add_association(tag_b, tag_a, weight, assoc_type);
    }

    /// Get all associations for a tag.
    pub fn get_associations(&self, tag: &Tag) -> &[Association] {
        self.associations.get(tag).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Get fact by ID.
    pub fn get_fact(&self, id: FactId) -> Option<&Fact> {
        self.facts.get(&id)
    }

    /// Get mutable fact by ID.
    pub fn get_fact_mut(&mut self, id: FactId) -> Option<&mut Fact> {
        self.facts.get_mut(&id)
    }

    /// Mark a fact as revealed.
    pub fn reveal_fact(&mut self, id: FactId) -> bool {
        if let Some(fact) = self.facts.get_mut(&id) {
            fact.reveal();
            true
        } else {
            false
        }
    }

    /// Get unrevealed secrets for an entity.
    pub fn unrevealed_secrets(&self, entity: EntityId) -> Vec<&Fact> {
        self.fact_by_entity
            .get(&entity)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.facts.get(id))
                    .filter(|fact| matches!(fact.fact_type, FactType::Secret { .. }) && !fact.revealed)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all facts in the knowledge base.
    pub fn all_facts(&self) -> impl Iterator<Item = &Fact> {
        self.facts.values()
    }

    /// Get the total number of facts.
    pub fn fact_count(&self) -> usize {
        self.facts.len()
    }

    /// Get all registered tags.
    pub fn all_tags(&self) -> impl Iterator<Item = &Tag> {
        self.tag_to_facts.keys()
    }

    /// Get the total number of tags.
    pub fn tag_count(&self) -> usize {
        self.tag_to_facts.len()
    }

    /// Check if a tag exists in the graph.
    pub fn has_tag(&self, tag: &Tag) -> bool {
        self.tag_to_facts.contains_key(tag)
    }

    /// Find facts matching a predicate.
    pub fn find_facts<F>(&self, predicate: F) -> Vec<&Fact>
    where
        F: Fn(&Fact) -> bool,
    {
        self.facts.values().filter(|f| predicate(f)).collect()
    }

    /// Get facts with importance above a threshold.
    pub fn important_facts(&self, threshold: f32) -> Vec<&Fact> {
        self.find_facts(|f| f.importance >= threshold)
    }

    /// Automatically create co-occurrence associations from shared tags on facts.
    pub fn build_co_occurrence_associations(&mut self) {
        // Collect pairs of tags that appear on the same fact
        let mut co_occurrences: HashMap<(Tag, Tag), u32> = HashMap::new();

        for fact in self.facts.values() {
            let tags: Vec<_> = fact.tags.iter().cloned().collect();
            for i in 0..tags.len() {
                for j in (i + 1)..tags.len() {
                    let key = if tags[i] < tags[j] {
                        (tags[i].clone(), tags[j].clone())
                    } else {
                        (tags[j].clone(), tags[i].clone())
                    };
                    *co_occurrences.entry(key).or_default() += 1;
                }
            }
        }

        // Create associations based on co-occurrence count
        for ((tag_a, tag_b), count) in co_occurrences {
            // Weight based on number of co-occurrences (log scale)
            let weight = (count as f32).ln().min(1.0);
            self.add_bidirectional_association(
                tag_a,
                tag_b,
                weight,
                AssociationType::CoOccurrence,
            );
        }
    }
}

// Implement PartialOrd for Tag to enable sorting in build_co_occurrence_associations
impl PartialOrd for Tag {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Tag {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_string().cmp(&other.as_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge_base::RelationshipType;

    #[test]
    fn test_add_and_get_fact() {
        let mut graph = KnowledgeGraph::new();

        let fact = Fact::new("The kingdom is at war")
            .with_tag(Tag::concept("War"))
            .with_tag(Tag::faction("Kingdom"));

        let id = graph.add_fact(fact);

        let retrieved = graph.get_fact(id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content, "The kingdom is at war");
    }

    #[test]
    fn test_facts_by_tag() {
        let mut graph = KnowledgeGraph::new();

        let war_tag = Tag::concept("War");

        graph.add_fact(Fact::new("Battle of the plains").with_tag(war_tag.clone()));
        graph.add_fact(Fact::new("Siege of the castle").with_tag(war_tag.clone()));
        graph.add_fact(Fact::new("Peace treaty signed").with_tag(Tag::concept("Peace")));

        let war_facts = graph.facts_by_tag(&war_tag);
        assert_eq!(war_facts.len(), 2);
    }

    #[test]
    fn test_associations() {
        let mut graph = KnowledgeGraph::new();

        let villain = Tag::concept("Villain");
        let magic = Tag::concept("Magic");
        let darkness = Tag::concept("Darkness");

        graph.add_association(villain.clone(), magic.clone(), 0.7, AssociationType::Direct);
        graph.add_association(villain.clone(), darkness.clone(), 0.9, AssociationType::Direct);

        let assocs = graph.get_associations(&villain);
        assert_eq!(assocs.len(), 2);

        // Check specific association
        let magic_assoc = assocs.iter().find(|a| a.target == magic);
        assert!(magic_assoc.is_some());
        assert!((magic_assoc.unwrap().weight - 0.7).abs() < 0.01);
    }

    #[test]
    fn test_bidirectional_associations() {
        let mut graph = KnowledgeGraph::new();

        let tag_a = Tag::concept("A");
        let tag_b = Tag::concept("B");

        graph.add_bidirectional_association(tag_a.clone(), tag_b.clone(), 0.8, AssociationType::Semantic);

        assert_eq!(graph.get_associations(&tag_a).len(), 1);
        assert_eq!(graph.get_associations(&tag_b).len(), 1);
    }

    #[test]
    fn test_facts_by_entity() {
        let mut graph = KnowledgeGraph::new();

        let entity_a = EntityId::new();
        let entity_b = EntityId::new();

        graph.add_fact(
            Fact::new("They are allies")
                .with_type(FactType::Relationship {
                    entity_a,
                    entity_b,
                    relationship: RelationshipType::Friend,
                    sentiment: 0.8,
                })
                .with_tag(Tag::entity(entity_a))
                .with_tag(Tag::entity(entity_b)),
        );

        graph.add_fact(Fact::new("Hero's trait").with_tag(Tag::entity(entity_a)));

        let entity_a_facts = graph.facts_by_entity(entity_a);
        assert_eq!(entity_a_facts.len(), 2);

        let entity_b_facts = graph.facts_by_entity(entity_b);
        assert_eq!(entity_b_facts.len(), 1);
    }

    #[test]
    fn test_reveal_fact() {
        let mut graph = KnowledgeGraph::new();

        let fact = Fact::new("Secret identity").with_known_to_player(false);
        let id = graph.add_fact(fact);

        assert!(!graph.get_fact(id).unwrap().revealed);
        assert!(!graph.get_fact(id).unwrap().known_to_player);

        graph.reveal_fact(id);

        assert!(graph.get_fact(id).unwrap().revealed);
        assert!(graph.get_fact(id).unwrap().known_to_player);
    }

    #[test]
    fn test_remove_fact() {
        let mut graph = KnowledgeGraph::new();

        let tag = Tag::concept("Test");
        let fact = Fact::new("Removable fact").with_tag(tag.clone());
        let id = graph.add_fact(fact);

        assert!(graph.get_fact(id).is_some());
        assert_eq!(graph.facts_by_tag(&tag).len(), 1);

        let removed = graph.remove_fact(id);
        assert!(removed.is_some());
        assert!(graph.get_fact(id).is_none());
        assert_eq!(graph.facts_by_tag(&tag).len(), 0);
    }

    #[test]
    fn test_important_facts() {
        let mut graph = KnowledgeGraph::new();

        graph.add_fact(Fact::new("Minor detail").with_importance(0.2));
        graph.add_fact(Fact::new("Important event").with_importance(0.8));
        graph.add_fact(Fact::new("Critical secret").with_importance(1.0));

        let important = graph.important_facts(0.7);
        assert_eq!(important.len(), 2);
    }

    #[test]
    fn test_co_occurrence_associations() {
        let mut graph = KnowledgeGraph::new();

        let magic = Tag::concept("Magic");
        let combat = Tag::concept("Combat");
        let villain = Tag::concept("Villain");

        // Two facts share magic and villain tags
        graph.add_fact(
            Fact::new("Villain casts spell")
                .with_tag(magic.clone())
                .with_tag(villain.clone()),
        );
        graph.add_fact(
            Fact::new("Villain summons demon")
                .with_tag(magic.clone())
                .with_tag(villain.clone()),
        );
        // One fact has magic and combat
        graph.add_fact(
            Fact::new("Combat spell")
                .with_tag(magic.clone())
                .with_tag(combat.clone()),
        );

        graph.build_co_occurrence_associations();

        // Magic-villain should have stronger association (2 co-occurrences)
        let magic_assocs = graph.get_associations(&magic);
        assert!(!magic_assocs.is_empty());

        let villain_assoc = magic_assocs.iter().find(|a| a.target == villain);
        let combat_assoc = magic_assocs.iter().find(|a| a.target == combat);

        assert!(villain_assoc.is_some());
        assert!(combat_assoc.is_some());
        // More co-occurrences = higher weight
        assert!(villain_assoc.unwrap().weight >= combat_assoc.unwrap().weight);
    }
}
