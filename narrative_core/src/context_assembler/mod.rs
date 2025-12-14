//! Context Assembler - Builds context for LLM prompts using spreading activation.
//!
//! The spreading activation algorithm works as follows:
//! 1. **Trigger**: Receive trigger tags from a game event
//! 2. **Activation**: Initialize trigger tags with energy
//! 3. **Spreading**: Energy spreads through tag associations
//! 4. **Filtering**: Collect "hot" tags above a threshold
//! 5. **Selection**: Retrieve facts associated with hot tags
//! 6. **Assembly**: Build structured context for LLM prompt

mod activation;

pub use activation::*;

use game_rules::{Season, Weather, WorldState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::events::GameEvent;
use crate::knowledge_base::{Fact, FactId, KnowledgeGraph, Tag};

/// Configuration for the spreading activation algorithm.
#[derive(Debug, Clone)]
pub struct ActivationConfig {
    /// Initial energy given to trigger tags.
    pub initial_energy: f32,

    /// How much energy decays at each step (0.0-1.0).
    pub decay_rate: f32,

    /// Maximum depth of spreading.
    pub max_depth: u32,

    /// Minimum energy threshold for inclusion.
    pub energy_threshold: f32,

    /// Maximum number of facts to include in context.
    pub max_facts: usize,
}

impl Default for ActivationConfig {
    fn default() -> Self {
        Self {
            initial_energy: 1.0,
            decay_rate: 0.5,
            max_depth: 2,
            energy_threshold: 0.1,
            max_facts: 20,
        }
    }
}

/// The context assembler builds prompts from knowledge and state.
pub struct ContextAssembler {
    config: ActivationConfig,
}

impl ContextAssembler {
    /// Create a new context assembler with the given configuration.
    pub fn new(config: ActivationConfig) -> Self {
        Self { config }
    }

    /// Create a context assembler with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(ActivationConfig::default())
    }

    /// Run spreading activation algorithm.
    ///
    /// # Algorithm
    ///
    /// 1. Initialize trigger tags with initial energy
    /// 2. For each depth level:
    ///    a. For each active tag, spread energy to associated tags
    ///    b. Energy is weighted by association strength and decay rate
    /// 3. Return activation state with all tag energies
    pub fn spread_activation(
        &self,
        graph: &KnowledgeGraph,
        trigger_tags: Vec<Tag>,
    ) -> ActivationState {
        let mut state = ActivationState::new();

        // Step 1: Initialize trigger tags
        for tag in trigger_tags {
            state.add_energy(tag, self.config.initial_energy);
        }

        // Step 2: Spread activation
        for _depth in 0..self.config.max_depth {
            let mut new_energies: HashMap<Tag, f32> = HashMap::new();

            for (tag, energy) in state.iter_energies() {
                if *energy < self.config.energy_threshold {
                    continue;
                }

                // Spread to associated tags
                for assoc in graph.get_associations(tag) {
                    let spread_energy = energy * assoc.weight * self.config.decay_rate;
                    *new_energies.entry(assoc.target.clone()).or_default() += spread_energy;
                }
            }

            // Merge new energies
            for (tag, energy) in new_energies {
                state.add_energy(tag, energy);
            }
        }

        state
    }

    /// Collect relevant facts based on activation state.
    ///
    /// Facts are scored by the sum of their tag energies multiplied by importance.
    pub fn collect_facts<'a>(
        &self,
        graph: &'a KnowledgeGraph,
        activation: &ActivationState,
    ) -> Vec<&'a Fact> {
        let hot_tags = activation.hot_tags(self.config.energy_threshold);

        let mut fact_scores: HashMap<FactId, f32> = HashMap::new();

        // Score facts by sum of their tag energies
        for (tag, energy) in hot_tags {
            for fact in graph.facts_by_tag(tag) {
                *fact_scores.entry(fact.id).or_default() += energy * fact.importance;
            }
        }

        // Sort by score and take top N
        let mut scored_facts: Vec<_> = fact_scores.into_iter().collect();
        scored_facts.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        scored_facts
            .into_iter()
            .take(self.config.max_facts)
            .filter_map(|(id, _)| graph.get_fact(id))
            .collect()
    }

    /// Assemble complete context for LLM prompt.
    pub fn assemble_context(
        &self,
        event: &GameEvent,
        graph: &KnowledgeGraph,
        world_state: &WorldState,
    ) -> AssembledContext {
        // Extract trigger tags from event
        let trigger_tags = self.extract_tags_from_event(event, world_state);

        // Run spreading activation
        let activation = self.spread_activation(graph, trigger_tags);

        // Collect relevant facts
        let facts = self.collect_facts(graph, &activation);

        // Build context sections
        AssembledContext {
            event_description: self.describe_event(event, world_state),
            relevant_facts: facts.iter().map(|f| f.content.clone()).collect(),
            world_context: self.extract_world_context(world_state),
            character_context: self.extract_character_context(world_state, event),
            activated_tags: activation
                .hot_tags(self.config.energy_threshold)
                .into_iter()
                .map(|(t, e)| (t.clone(), e))
                .collect(),
        }
    }

    /// Extract tags from a game event.
    fn extract_tags_from_event(&self, event: &GameEvent, world_state: &WorldState) -> Vec<Tag> {
        let mut tags = Vec::new();

        // Add event-specific tags
        tags.extend(event.to_tags());

        // Add current location tag for the primary entity
        if let Some(entity_id) = event.primary_entity() {
            if let Some(loc_id) = world_state.entity_locations.get(&entity_id) {
                tags.push(Tag::location(*loc_id));
            }
        }

        tags
    }

    /// Describe an event in human-readable form.
    fn describe_event(&self, event: &GameEvent, world_state: &WorldState) -> String {
        match event {
            GameEvent::CombatAbilityUsed {
                source,
                target,
                ability,
            } => {
                let source_name = world_state
                    .get_character(*source)
                    .map(|c| c.name.as_str())
                    .unwrap_or("Unknown");
                let target_name = world_state
                    .get_character(*target)
                    .map(|c| c.name.as_str())
                    .unwrap_or("Unknown");
                format!(
                    "{} used ability '{}' on {}",
                    source_name, ability, target_name
                )
            }
            GameEvent::DialogueStarted { participants, topic } => {
                let names: Vec<_> = participants
                    .iter()
                    .filter_map(|id| world_state.get_character(*id))
                    .map(|c| c.name.as_str())
                    .collect();
                match topic {
                    Some(t) => format!("{} begin discussing {}", names.join(" and "), t),
                    None => format!("{} begin a conversation", names.join(" and ")),
                }
            }
            GameEvent::LocationEntered { entity, location } => {
                let entity_name = world_state
                    .get_character(*entity)
                    .map(|c| c.name.as_str())
                    .unwrap_or("Someone");
                let loc_name = world_state
                    .locations
                    .get(location)
                    .map(|l| l.name.as_str())
                    .unwrap_or("an unknown place");
                format!("{} enters {}", entity_name, loc_name)
            }
            GameEvent::EntityDied { entity, killer } => {
                let entity_name = world_state
                    .get_character(*entity)
                    .map(|c| c.name.as_str())
                    .unwrap_or("Someone");
                match killer {
                    Some(k) => {
                        let killer_name = world_state
                            .get_character(*k)
                            .map(|c| c.name.as_str())
                            .unwrap_or("an unknown assailant");
                        format!("{} was killed by {}", entity_name, killer_name)
                    }
                    None => format!("{} has died", entity_name),
                }
            }
            _ => format!("{:?}", event),
        }
    }

    /// Extract world context information.
    fn extract_world_context(&self, world_state: &WorldState) -> WorldContext {
        WorldContext {
            time_of_day: format!(
                "{:02}:{:02}",
                world_state.time.hour, world_state.time.minute
            ),
            day: world_state.time.day,
            season: world_state.time.season,
            weather: world_state.environment.weather,
            is_night: world_state.is_night(),
        }
    }

    /// Extract character context for involved entities.
    fn extract_character_context(
        &self,
        world_state: &WorldState,
        event: &GameEvent,
    ) -> Vec<CharacterContext> {
        event
            .involved_entities()
            .iter()
            .filter_map(|id| world_state.get_character(*id))
            .map(|c| CharacterContext {
                name: c.name.clone(),
                title: c.title.clone(),
                current_hp_percent: if c.stats.max_hp > 0 {
                    (c.stats.current_hp as f32 / c.stats.max_hp as f32 * 100.0) as u32
                } else {
                    100
                },
                active_statuses: c
                    .status_effects
                    .active_effects
                    .iter()
                    .map(|e| format!("{:?}", e.effect_type))
                    .collect(),
                personality: c.personality_traits.clone(),
            })
            .collect()
    }
}

/// The assembled context ready for prompt generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembledContext {
    /// Description of the triggering event.
    pub event_description: String,

    /// Relevant facts from the knowledge base.
    pub relevant_facts: Vec<String>,

    /// World state context.
    pub world_context: WorldContext,

    /// Context for involved characters.
    pub character_context: Vec<CharacterContext>,

    /// Tags that were activated with their energy levels.
    pub activated_tags: Vec<(Tag, f32)>,
}

impl AssembledContext {
    /// Format the context as a prompt string.
    pub fn to_prompt_string(&self) -> String {
        let mut prompt = String::new();

        prompt.push_str("## Current Event\n");
        prompt.push_str(&self.event_description);
        prompt.push_str("\n\n");

        prompt.push_str("## World State\n");
        prompt.push_str(&format!(
            "Time: {} (Day {}), {} {:?}\n",
            self.world_context.time_of_day,
            self.world_context.day,
            if self.world_context.is_night {
                "Night"
            } else {
                "Day"
            },
            self.world_context.weather
        ));
        prompt.push('\n');

        if !self.character_context.is_empty() {
            prompt.push_str("## Involved Characters\n");
            for char in &self.character_context {
                prompt.push_str(&format!(
                    "- {}{}: HP {}%, Conditions: {}\n",
                    char.name,
                    char.title
                        .as_ref()
                        .map(|t| format!(", {}", t))
                        .unwrap_or_default(),
                    char.current_hp_percent,
                    if char.active_statuses.is_empty() {
                        "None".to_string()
                    } else {
                        char.active_statuses.join(", ")
                    }
                ));
            }
            prompt.push('\n');
        }

        if !self.relevant_facts.is_empty() {
            prompt.push_str("## Relevant Background\n");
            for fact in &self.relevant_facts {
                prompt.push_str(&format!("- {}\n", fact));
            }
            prompt.push('\n');
        }

        prompt
    }
}

/// World context for LLM prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldContext {
    pub time_of_day: String,
    pub day: u32,
    pub season: Season,
    pub weather: Weather,
    pub is_night: bool,
}

/// Character context for LLM prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterContext {
    pub name: String,
    pub title: Option<String>,
    pub current_hp_percent: u32,
    pub active_statuses: Vec<String>,
    pub personality: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge_base::{AssociationType, Fact};
    use game_rules::{Character, EntityId};

    fn setup_test_graph() -> KnowledgeGraph {
        let mut graph = KnowledgeGraph::new();

        // Create some entities
        let villain = EntityId::new();
        let priestess = EntityId::new();

        // Add tags and associations
        let villain_tag = Tag::entity(villain);
        let priestess_tag = Tag::entity(priestess);
        let magic_tag = Tag::concept("Magic");
        let orphanage_tag = Tag::concept("Orphanage");

        // Add associations
        graph.add_association(villain_tag.clone(), orphanage_tag.clone(), 0.9, AssociationType::Direct);
        graph.add_association(priestess_tag.clone(), orphanage_tag.clone(), 0.9, AssociationType::Direct);
        graph.add_association(villain_tag.clone(), magic_tag.clone(), 0.7, AssociationType::Direct);
        graph.add_association(priestess_tag.clone(), magic_tag.clone(), 0.8, AssociationType::Direct);

        // Add facts
        graph.add_fact(
            Fact::new("The villain and priestess grew up in Morning Star orphanage")
                .with_tag(villain_tag.clone())
                .with_tag(priestess_tag.clone())
                .with_tag(orphanage_tag.clone())
                .with_importance(0.9),
        );

        graph.add_fact(
            Fact::new("Both trained in magical arts from a young age")
                .with_tag(villain_tag)
                .with_tag(priestess_tag)
                .with_tag(magic_tag)
                .with_importance(0.7),
        );

        graph
    }

    #[test]
    fn test_spreading_activation() {
        let graph = setup_test_graph();
        let assembler = ContextAssembler::with_defaults();

        let trigger = vec![Tag::concept("Orphanage")];
        let activation = assembler.spread_activation(&graph, trigger);

        // Orphanage should have high energy (trigger)
        assert!(activation.get_energy(&Tag::concept("Orphanage")) >= 0.9);
    }

    #[test]
    fn test_activation_decay() {
        let mut graph = KnowledgeGraph::new();

        let tag_a = Tag::concept("A");
        let tag_b = Tag::concept("B");
        let tag_c = Tag::concept("C");

        // Use a weaker association weight so decay is more visible
        graph.add_association(tag_a.clone(), tag_b.clone(), 0.8, AssociationType::Direct);
        graph.add_association(tag_b.clone(), tag_c.clone(), 0.8, AssociationType::Direct);

        let config = ActivationConfig {
            initial_energy: 1.0,
            decay_rate: 0.5,
            max_depth: 1, // Single depth to show clear decay
            energy_threshold: 0.01,
            max_facts: 10,
        };

        let assembler = ContextAssembler::new(config);
        let activation = assembler.spread_activation(&graph, vec![tag_a.clone()]);

        // Energy should decay with each hop
        let energy_a = activation.get_energy(&tag_a);
        let energy_b = activation.get_energy(&tag_b);

        // With depth=1: A=1.0, B=1.0*0.8*0.5=0.4, C gets no energy (needs depth 2)
        assert!(
            energy_a > energy_b,
            "energy_a ({}) should be > energy_b ({})",
            energy_a,
            energy_b
        );
        assert!(energy_b > 0.0, "energy_b ({}) should be > 0", energy_b);
    }

    #[test]
    fn test_activation_multi_hop() {
        let mut graph = KnowledgeGraph::new();

        let tag_a = Tag::concept("A");
        let tag_b = Tag::concept("B");
        let tag_c = Tag::concept("C");

        graph.add_association(tag_a.clone(), tag_b.clone(), 0.8, AssociationType::Direct);
        graph.add_association(tag_b.clone(), tag_c.clone(), 0.8, AssociationType::Direct);

        let config = ActivationConfig {
            initial_energy: 1.0,
            decay_rate: 0.5,
            max_depth: 2, // Two depth levels
            energy_threshold: 0.01,
            max_facts: 10,
        };

        let assembler = ContextAssembler::new(config);
        let activation = assembler.spread_activation(&graph, vec![tag_a.clone()]);

        // With depth=2, energy spreads further
        let energy_a = activation.get_energy(&tag_a);
        let energy_b = activation.get_energy(&tag_b);
        let energy_c = activation.get_energy(&tag_c);

        // All should have some energy
        assert!(energy_a > 0.0);
        assert!(energy_b > 0.0);
        assert!(energy_c > 0.0);

        // Energy should generally decrease with distance (though B accumulates)
        // The key insight is that C gets energy proportional to B's energy
        assert!(energy_b > energy_c, "energy_b ({}) should be > energy_c ({})", energy_b, energy_c);
    }

    #[test]
    fn test_collect_facts() {
        let graph = setup_test_graph();
        let assembler = ContextAssembler::with_defaults();

        let trigger = vec![Tag::concept("Orphanage")];
        let activation = assembler.spread_activation(&graph, trigger);
        let facts = assembler.collect_facts(&graph, &activation);

        // Should find the orphanage fact
        assert!(!facts.is_empty());
        assert!(facts
            .iter()
            .any(|f| f.content.contains("Morning Star orphanage")));
    }

    #[test]
    fn test_assemble_context() {
        let mut graph = setup_test_graph();
        let mut world_state = WorldState::new();

        // Add characters to world state
        let hero = Character::new("Hero");
        let hero_id = world_state.add_character(hero);

        graph.add_fact(
            Fact::new("The hero is brave and just")
                .with_tag(Tag::entity(hero_id))
                .with_importance(0.6),
        );

        let assembler = ContextAssembler::with_defaults();

        let event = GameEvent::LocationEntered {
            entity: hero_id,
            location: game_rules::LocationId::new(),
        };

        let context = assembler.assemble_context(&event, &graph, &world_state);

        assert!(!context.event_description.is_empty());
    }

    #[test]
    fn test_context_to_prompt() {
        let context = AssembledContext {
            event_description: "Hero enters the dark forest".to_string(),
            relevant_facts: vec![
                "The forest is haunted".to_string(),
                "Dangerous creatures lurk within".to_string(),
            ],
            world_context: WorldContext {
                time_of_day: "22:00".to_string(),
                day: 42,
                season: Season::Autumn,
                weather: Weather::Foggy,
                is_night: true,
            },
            character_context: vec![CharacterContext {
                name: "Hero".to_string(),
                title: Some("The Brave".to_string()),
                current_hp_percent: 80,
                active_statuses: vec![],
                personality: vec!["brave".to_string()],
            }],
            activated_tags: vec![],
        };

        let prompt = context.to_prompt_string();

        assert!(prompt.contains("Hero enters the dark forest"));
        assert!(prompt.contains("22:00"));
        assert!(prompt.contains("Night"));
        assert!(prompt.contains("forest is haunted"));
        assert!(prompt.contains("HP 80%"));
    }
}
