# The Cortex: Implementation Roadmap

## Executive Summary

This roadmap provides a step-by-step guide to build **The Cortex** from scratch: an AI-driven narrative engine for a DnD 5e roguelike game. The system uses a local LLM (Ministral-3-14B via llama.cpp) with spreading activation RAG for context-aware storytelling.

**Target Hardware:** AMD RX 9070 XT (16GB VRAM)
**Core Technology Stack:** Rust, llama.cpp, Bevy ECS, GGUF models
**Design Philosophy:** State-driven architecture, event-based communication, deterministic where possible

---

## Prerequisites & Development Environment

### Hardware Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| GPU VRAM | 12GB | 16GB (RX 9070 XT) |
| System RAM | 16GB | 32GB |
| Storage | 50GB SSD | 100GB NVMe |
| CPU | AVX2 support | 8+ cores |

### Software Requirements

```bash
# Rust toolchain (stable)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable

# llama.cpp (with ROCm for AMD)
git clone https://github.com/ggml-org/llama.cpp
cd llama.cpp && make LLAMA_HIPBLAS=1

# Model download
huggingface-cli download mistralai/Ministral-3-14B-Reasoning-2512-GGUF \
  --include "*Q5_K_M*" --local-dir ./models
```

### Key Dependencies

| Crate | Purpose | Version |
|-------|---------|---------|
| `llama_cpp` | LLM inference bindings | Latest |
| `bevy` | Game engine (Phase 5) | 0.15+ |
| `serde` / `serde_json` | Serialization | 1.x |
| `tokio` | Async runtime | 1.x |
| `indexmap` | Deterministic HashMap | 2.x |
| `schemars` | JSON Schema generation | 0.8+ |
| `rust-i18n` | Localization | 3.x |

---

## Phase 1: Proof of Concept — Core Narrative Loop

**Goal:** Player action → AI narrative response (CLI text game, no persistence)

**Duration:** Foundation milestone
**Deliverable:** Interactive text game demonstrating LLM-powered storytelling

### 1.1 Project Structure Setup

```
cortex/
├── Cargo.toml              # Workspace manifest
├── crates/
│   ├── dnd_rules/          # DnD 5e mechanics
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── types/      # Core types (EntityId, Pool, AbilityScores)
│   │       ├── bestiary/   # Creature templates
│   │       └── effects/    # Conditions, status effects
│   └── narrative_core/     # The Cortex brain
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── llm_interface/
│           ├── knowledge_base/
│           ├── context_assembler/
│           └── event_processor/
├── data/
│   ├── config/
│   ├── bestiary/
│   ├── locales/
│   └── prompts/
└── examples/
    └── cli_game.rs         # Phase 1 demo
```

### 1.2 Implementation Tasks

#### Task 1.2.1: Core Types (`dnd_rules/src/types/`)

```rust
// Priority types to implement first:
pub struct EntityId(pub u32);
pub struct Pool { pub current: u32, pub max: u32 }
pub struct AbilityScores { pub str: u8, pub dex: u8, ... }
pub enum DamageType { Slashing, Piercing, Fire, ... }
pub struct DiceExpr { pub count: u8, pub sides: u8, pub modifier: i8 }
```

**Acceptance Criteria:**
- [ ] All types derive `Debug, Clone, Serialize, Deserialize`
- [ ] `Pool::damage()` and `Pool::heal()` use saturating arithmetic
- [ ] `DiceExpr::parse()` handles "2d6+3" notation
- [ ] Unit tests for all methods

#### Task 1.2.2: LLM Client (`narrative_core/src/llm_interface/`)

```rust
pub struct OllamaClient {
    client: reqwest::Client,
    config: OllamaConfig,
}

impl OllamaClient {
    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, LLMError>;
}
```

**Integration with llama.cpp:**
- Use `llama_cpp` crate for direct GGUF loading (preferred for performance)
- Alternative: HTTP API via Ollama for simpler setup
- Configure for Ministral-3-14B Q5_K_M (~10.5GB VRAM)

**Acceptance Criteria:**
- [ ] Successful model loading with Q5_K_M quantization
- [ ] Response latency <3s for typical prompts
- [ ] Proper error handling for timeouts and connection issues
- [ ] VRAM usage stable under 15GB

#### Task 1.2.3: Basic Prompt Template

```rust
const SYSTEM_PROMPT: &str = r#"
You are the AI Dungeon Master for a DnD 5e roguelike game.
Generate immersive narrative responses based on player actions.
Respond in vivid, engaging prose appropriate for fantasy RPG.
"#;

pub fn build_narrative_prompt(action: &str, context: &str) -> String {
    format!("## Context\n{}\n\n## Player Action\n{}\n\nNarrate what happens:", context, action)
}
```

#### Task 1.2.4: CLI Game Loop

```rust
// examples/cli_game.rs
#[tokio::main]
async fn main() {
    let llm = OllamaClient::new(OllamaConfig::default());
    let mut context = "You stand at the entrance of a dark dungeon.".to_string();

    loop {
        println!("\n{}\n", context);
        print!("> ");
        let action = read_line();

        let response = llm.generate_narrative(&action, &context).await?;
        context = response.narrative_text;
    }
}
```

### 1.3 Validation Criteria

| Metric | Target | How to Measure |
|--------|--------|----------------|
| Response latency | <3s | `std::time::Instant` |
| VRAM usage | <15GB | `nvidia-smi` / `rocm-smi` |
| Prose quality | Subjective | Manual playtesting |
| Crash rate | 0% | Extended play session |

---

## Phase 2: Structured Output & Fact Extraction

**Goal:** AI extracts and returns structured data alongside narrative

**Duration:** Critical foundation for RAG
**Deliverable:** Two-pass generation with GBNF-constrained JSON extraction

### 2.1 JSON Schema Definition

```rust
// narrative_core/src/llm_interface/response_types.rs

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractionOutput {
    /// Facts worth remembering from this scene
    pub facts: Vec<ExtractedFact>,

    /// Any relationship changes between entities
    pub relationship_changes: Vec<RelationshipChange>,

    /// State changes to apply
    pub state_changes: Vec<StateChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractedFact {
    /// Natural language description
    pub content: String,

    /// Importance score (0.0 - 1.0)
    pub importance: f32,

    /// Tags for knowledge graph indexing
    pub tags: Vec<String>,

    /// Is this a secret not known to player?
    pub is_secret: bool,
}
```

### 2.2 GBNF Grammar Generation

**Best Practice:** Use `schemars` to auto-generate JSON Schema, then convert to GBNF.

```rust
use schemars::schema_for;

fn generate_extraction_grammar() -> String {
    let schema = schema_for!(ExtractionOutput);
    json_schema_to_gbnf(&schema) // Use llama.cpp's converter
}
```

**Important Considerations:**
1. Inform the model of expected schema in the prompt (grammar enforces syntax, not semantics)
2. Handle token limits - grammar doesn't prevent incomplete JSON
3. Test grammars with `llama-gbnf-validator`
4. Avoid complex optional field patterns - use placeholder values instead

### 2.3 Two-Pass Generation Pipeline

```
Player Action
    ↓
[Pass 1: Narrative Generation]
    Input: Game state + action
    Output: Prose narrative
    ↓
[Pass 2: Fact Extraction]
    Input: Narrative + extraction schema
    Output: GBNF-constrained JSON
    ↓
Parsed Facts + Narrative
```

```rust
pub async fn generate_with_extraction(
    &self,
    action: &str,
    context: &AssembledContext,
) -> Result<(String, ExtractionOutput), Error> {
    // Pass 1: Generate narrative (no grammar constraint)
    let narrative = self.generate_narrative(action, context).await?;

    // Pass 2: Extract facts (GBNF-constrained)
    let extraction_prompt = format!(
        "Analyze the following scene and extract important facts.\n\nScene:\n{}\n\nExtract facts as JSON:",
        narrative
    );

    let extraction = self.generate_with_grammar(
        &extraction_prompt,
        &self.extraction_grammar,
    ).await?;

    let parsed: ExtractionOutput = serde_json::from_str(&extraction)?;
    Ok((narrative, parsed))
}
```

### 2.4 Validation Criteria

| Metric | Target | How to Measure |
|--------|--------|----------------|
| JSON validity | 100% | GBNF guarantees syntax |
| Fact relevance | >80% useful | Manual review of 50 scenes |
| Combined latency | <6s | Both passes complete |
| Parse errors | 0% | Logging + monitoring |

---

## Phase 3: Knowledge Graph & Spreading Activation RAG

**Goal:** Context-aware generation using accumulated world knowledge

**Duration:** Core differentiator of the system
**Deliverable:** Working knowledge graph with spreading activation retrieval

### 3.1 Knowledge Graph Implementation

**Key Design Decision:** Use `IndexMap` instead of `HashMap` for deterministic iteration.

```rust
// narrative_core/src/knowledge_base/graph.rs

use indexmap::IndexMap;

pub struct KnowledgeGraph {
    /// All facts stored by ID
    facts: IndexMap<FactId, Fact>,

    /// Tag -> Facts index
    tag_to_facts: IndexMap<Tag, IndexSet<FactId>>,

    /// Tag associations (weighted edges)
    associations: IndexMap<Tag, Vec<Association>>,
}
```

**Tag Types (strictly defined - no Custom variant):**
```rust
pub enum Tag {
    Entity(EntityId),      // Characters, creatures, items
    Location(u32),         // World locations
    Concept(String),       // Themes: "betrayal", "magic"
    Faction(String),       // Organizations: "Crown", "Thieves Guild"
    EventType(String),     // Categories: "combat", "dialogue"
    RelationType(String),  // "friendship", "rivalry"
}
```

### 3.2 Spreading Activation Algorithm

**Algorithm Overview:**
1. Initialize trigger tags with energy 1.0
2. For each depth level, spread energy to associated tags
3. Apply decay at each step
4. Collect all tags above threshold
5. Retrieve facts tagged with high-energy tags

```rust
pub fn spread_activation(
    &self,
    graph: &KnowledgeGraph,
    trigger_tags: Vec<Tag>,
) -> ActivationState {
    let mut state = ActivationState::new();

    // Initialize triggers
    for tag in trigger_tags {
        state.add_energy(tag, self.config.initial_energy);
    }

    // Spread for max_depth iterations
    for _depth in 0..self.config.max_depth {
        let mut new_energies = IndexMap::new();

        for (tag, energy) in &state.energies {
            if *energy < self.config.energy_threshold {
                continue;
            }

            for assoc in graph.get_associations(tag) {
                let spread = energy * assoc.weight * self.config.decay_rate;
                *new_energies.entry(assoc.target.clone()).or_default() += spread;
            }
        }

        for (tag, energy) in new_energies {
            state.add_energy(tag, energy);
        }
    }

    state
}
```

**Default Configuration:**
```rust
ActivationConfig {
    initial_energy: 1.0,
    decay_rate: 0.5,
    max_depth: 2,
    energy_threshold: 0.1,
    max_facts: 20,
}
```

### 3.3 Context Assembly

```rust
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

    // Collect facts by combined score (activation * importance)
    let facts = self.collect_facts(graph, &activation);

    // Build context with token budget
    self.build_context_within_budget(facts, world_state, MAX_CONTEXT_TOKENS)
}
```

### 3.4 Token Budget Management

```rust
const MAX_CONTEXT_TOKENS: usize = 4000; // Leave room for response

fn build_context_within_budget(
    &self,
    facts: Vec<&Fact>,
    world_state: &WorldState,
    budget: usize,
) -> AssembledContext {
    let mut used_tokens = 0;
    let mut included_facts = Vec::new();

    // Reserve tokens for world context (~200)
    let world_context = self.format_world_context(world_state);
    used_tokens += estimate_tokens(&world_context);

    // Add facts in priority order until budget exhausted
    for fact in facts.iter().sorted_by(|a, b|
        b.importance.total_cmp(&a.importance)
    ) {
        let fact_tokens = estimate_tokens(&fact.content);
        if used_tokens + fact_tokens > budget {
            break;
        }
        included_facts.push(fact.content.clone());
        used_tokens += fact_tokens;
    }

    AssembledContext {
        relevant_facts: included_facts,
        world_context,
        // ...
    }
}
```

### 3.5 Persistence Layer

**Primary Storage:** Binary (bincode) for performance
**Secondary Storage:** JSON for debugging and editing

```rust
pub trait KnowledgeStorage {
    fn save(&self, graph: &KnowledgeGraph) -> Result<(), StorageError>;
    fn load(&self) -> Result<KnowledgeGraph, StorageError>;
}

// BinaryStorage using bincode - fast and compact
// JsonStorage using serde_json - human-readable
```

### 3.6 Validation Criteria

| Metric | Target | How to Measure |
|--------|--------|----------------|
| Determinism | Same seed → same results | Seeded test runs |
| Fact retrieval relevance | References past events | Manual review |
| Token budget adherence | No overflow | Assertion checks |
| Persistence round-trip | No data loss | Save/load tests |

---

## Phase 4: Game Mechanics & Combat

**Goal:** Integrate D&D 5e SRD combat with AI-driven tactical decisions

**Duration:** Gameplay milestone
**Deliverable:** Functional turn-based combat encounters

### 4.1 Bestiary System

**Data Format:** TOML files loaded at runtime

```toml
# data/bestiary/goblin.toml
id = "goblin"
name = "creature.goblin.name"
description = "creature.goblin.description"

creature_type = { Humanoid = ["goblinoid"] }
size = "Small"
ac = 15
hit_dice = { count = 2, sides = 6, modifier = 0 }
cr = { value = 0.25, xp = 50 }

[abilities]
str = 8
dex = 14
con = 10
int = 10
wis = 8
cha = 8

[speed]
walk = 30

[[actions]]
name = "action.scimitar"
attack_bonus = 4
damage = { count = 1, sides = 6, modifier = 2 }
damage_type = "Slashing"
reach_or_range = "5 ft."
```

**Localization:** Use `rust-i18n` with locale keys in TOML.

```yaml
# data/locales/en.yaml
creature:
  goblin:
    name: "Goblin"
    description: "A small, green-skinned creature wielding a rusty shortsword."
```

### 4.2 Combat AI Response Type

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CombatResponse {
    /// Selected action
    pub action: CombatAction,

    /// Target entity ID
    pub target: EntityId,

    /// AI reasoning (for debugging/narrative)
    pub reasoning: Option<String>,

    /// Tool calls (record facts about combat)
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum CombatAction {
    Attack { ability: String },
    Defend,
    UseItem { item_id: EntityId },
    Flee,
    LegendaryAction { name: String },
}
```

### 4.3 Combat Loop Integration

```rust
pub async fn process_combat_turn(
    &mut self,
    combatant: EntityId,
    enemies: &[EntitySnapshot],
    world_state: &WorldState,
) -> Result<CombatResponse, Error> {
    // Build combat context
    let context = self.assemble_combat_context(combatant, enemies, world_state);

    // Generate AI decision
    let decision = self.llm.generate_combat_decision(&context).await?;

    // Validate action is legal
    self.validate_combat_action(&decision, combatant)?;

    Ok(decision)
}
```

### 4.4 Encounter Generation

**Lazy Generation Pattern:**
- Load base creature template from bestiary
- AI generates unique names, descriptions, personality on first encounter
- Cache generated details for consistency

```rust
pub struct EncounterResponse {
    pub creatures: Vec<EncounterCreature>,
    pub introduction: String,
    pub difficulty: EncounterDifficulty,
}

pub struct EncounterCreature {
    pub template_id: String,  // From bestiary
    pub count: u8,
}
```

### 4.5 Validation Criteria

| Metric | Target | How to Measure |
|--------|--------|----------------|
| SRD compliance | Core rules correct | Manual verification |
| AI tactical quality | Reasonable decisions | Playtesting |
| Combat latency | <5s per turn | Timing |
| Creature variety | 20+ SRD creatures | Count bestiary files |

---

## Phase 5: Bevy Integration & Full Game

**Goal:** Complete game engine integration with persistence

**Duration:** Production milestone
**Deliverable:** Playable game prototype

### 5.1 Bevy ECS Integration Strategy

**Phase 5a: Sync Layer (Recommended Start)**

Keep `WorldState` as primary, sync bidirectionally with Bevy:

```rust
// Bevy → Cortex sync
pub fn sync_bevy_to_cortex(
    mut cortex_state: ResMut<WorldState>,
    query: Query<(&CortexEntityId, &Transform), Changed<Transform>>,
) {
    for (id, transform) in query.iter() {
        cortex_state.update_entity_location(id.0, transform.translation);
    }
}

// Cortex → Bevy sync
pub fn sync_cortex_to_bevy(
    cortex_state: Res<WorldState>,
    mut query: Query<(&CortexEntityId, &mut Health)>,
) {
    for (id, mut health) in query.iter_mut() {
        if let Some(char) = cortex_state.get_character(id.0) {
            health.current = char.stats.current_hp;
        }
    }
}
```

**Phase 5b: Native Components (Future Evolution)**

Eventually, `dnd_rules` types become Bevy components directly:

```rust
#[derive(Component)]
pub struct DndStats(pub AbilityScores);

#[derive(Component)]
pub struct DndHitPoints(pub Pool);
```

### 5.2 Async LLM Integration

**Challenge:** Bevy is sync, LLM calls are async.

**Solution:** Use channels for communication:

```rust
#[derive(Resource)]
pub struct NarrativeChannel {
    pub request_tx: mpsc::Sender<NarrativeRequest>,
    pub response_rx: mpsc::Receiver<NarrativeResponse>,
}

// Background task handles async LLM calls
pub fn spawn_narrative_worker(
    runtime: tokio::runtime::Runtime,
    processor: EventProcessor,
    rx: mpsc::Receiver<NarrativeRequest>,
    tx: mpsc::Sender<NarrativeResponse>,
) {
    std::thread::spawn(move || {
        runtime.block_on(async {
            while let Ok(request) = rx.recv() {
                let response = processor.process_event(request.event).await;
                tx.send(response).ok();
            }
        });
    });
}
```

### 5.3 Save System

```rust
#[derive(Serialize, Deserialize)]
pub struct SaveData {
    pub version: u32,
    pub world_state: WorldState,
    pub knowledge_graph: KnowledgeGraph,
    pub player_progress: PlayerProgress,
    pub timestamp: DateTime<Utc>,
}

pub fn save_game(path: &Path, data: &SaveData) -> Result<(), SaveError> {
    let json = serde_json::to_string_pretty(data)?;
    std::fs::write(path, json)?;
    Ok(())
}
```

### 5.4 UI Integration

**Dialogue System:**
- AI generates 2-4 player options per turn
- Display with emotional tone hints
- Track conversation state via knowledge graph

```rust
pub struct DialogueResponse {
    pub npc_text: String,
    pub tone: EmotionalTone,
    pub player_variants: Vec<DialogVariant>,
}

pub struct DialogVariant {
    pub text: String,
    pub tone: EmotionalTone,
    pub consequence_hint: Option<String>,
}
```

### 5.5 Validation Criteria

| Metric | Target | How to Measure |
|--------|--------|----------------|
| Save/load integrity | No data loss | Round-trip tests |
| Frame rate | 60 FPS during LLM wait | Bevy diagnostics |
| Memory stability | No leaks | Long play session |
| Player experience | Engaging gameplay | User testing |

---

## Technical Debt & Quality Gates

### Code Quality Standards

```rust
// Required for all types
#[derive(Debug, Clone, Serialize, Deserialize)]

// Use IndexMap for deterministic traversal
use indexmap::IndexMap;

// Use total_cmp for f32 sorting
facts.sort_by(|a, b| b.importance.total_cmp(&a.importance));

// Define proper error types
#[derive(Debug, thiserror::Error)]
pub enum CortexError {
    #[error("LLM error: {0}")]
    Llm(#[from] LLMError),
    // ...
}
```

### Testing Strategy

| Level | Tools | Coverage Target |
|-------|-------|-----------------|
| Unit | `#[test]`, proptest | Core logic 80% |
| Integration | mockall, tokio::test | LLM interactions |
| Snapshot | insta | Prompt templates |
| E2E | CLI test harness | Critical paths |

### Performance Monitoring

```rust
// Add timing instrumentation
let start = std::time::Instant::now();
let response = llm.generate(prompt).await?;
let duration = start.elapsed();
tracing::info!(duration_ms = duration.as_millis(), "LLM generation");
```

---

## Risk Mitigation

### Technical Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| LLM latency too high | Medium | High | Async processing, preloading |
| VRAM overflow | Low | High | Monitor usage, fallback to Q4_K_M |
| GBNF parse failures | Low | Medium | Robust error handling, retries |
| Knowledge graph explosion | Medium | Medium | Fact expiration, importance pruning |

### Dependency Risks

| Dependency | Risk | Mitigation |
|------------|------|------------|
| llama.cpp | Breaking changes | Pin version, integration tests |
| Mistral model | Availability | Local model storage, alternatives |
| Bevy | API changes | Follow upgrade guides, abstraction layer |

---

## Success Metrics

### Phase Completion Criteria

| Phase | Core Deliverable | Must Have |
|-------|------------------|-----------|
| 1 | CLI text game | <3s latency, stable VRAM |
| 2 | Fact extraction | 100% valid JSON, >80% relevance |
| 3 | Knowledge RAG | Deterministic, references history |
| 4 | Combat system | SRD-compliant, AI tactics |
| 5 | Full game | Playable prototype, save/load |

### Overall Project Success

- **Technical:** All phases complete with validation criteria met
- **User Experience:** Engaging AI-driven narrative gameplay
- **Performance:** Smooth local execution on target hardware
- **Maintainability:** Clean architecture, comprehensive tests

---

## Appendix A: File Structure Reference

```
cortex/
├── Cargo.toml
├── crates/
│   ├── dnd_rules/
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── types/mod.rs
│   │       ├── bestiary/mod.rs
│   │       └── effects/mod.rs
│   └── narrative_core/
│       └── src/
│           ├── lib.rs
│           ├── config.rs
│           ├── knowledge_base/
│           │   ├── mod.rs
│           │   ├── types.rs
│           │   ├── graph.rs
│           │   └── storage.rs
│           ├── context_assembler/mod.rs
│           ├── llm_interface/
│           │   ├── mod.rs
│           │   ├── response_types.rs
│           │   ├── ollama.rs
│           │   └── tools/
│           └── event_processor/mod.rs
├── data/
│   ├── config/
│   │   ├── cortex.toml
│   │   └── llm.toml
│   ├── bestiary/
│   │   ├── humanoids/
│   │   └── beasts/
│   ├── locales/
│   │   ├── en.yaml
│   │   └── ru.yaml
│   └── prompts/
│       ├── system.txt
│       └── combat.txt
├── examples/
│   └── cli_game.rs
└── tests/
    ├── knowledge_base_tests.rs
    ├── integration_tests.rs
    └── prompt_tests.rs
```

---

## Appendix B: Research Sources

### LLM & llama.cpp
- [llama.cpp GitHub](https://github.com/ggml-org/llama.cpp) - Core inference engine
- [llama_cpp-rs](https://github.com/edgenai/llama_cpp-rs) - High-level Rust bindings
- [GBNF Grammar Guide](https://github.com/ggml-org/llama.cpp/blob/master/grammars/README.md) - Structured output

### Knowledge Graphs & RAG
- [Spreading Activation for RAG](https://arxiv.org/abs/2512.15922) - Academic foundation
- [GraphRAG-RS](https://github.com/automataIA/graphrag-rs) - Rust implementation reference

### Game Development
- [Bevy Engine](https://bevyengine.org/) - ECS game engine
- [Open5e API](https://open5e.com/) - D&D 5e SRD reference
- [The Red Prison](https://store.steampowered.com/app/1074040/The_Red_Prison/) - 5e roguelike reference

### Model Selection
- [Ministral-3-14B](https://huggingface.co/mistralai/Ministral-3-14B-Reasoning-2512) - Recommended model
- [Mistral 3 Announcement](https://mistral.ai/news/mistral-3) - Benchmark data

---

*This roadmap is a living document. Update as implementation progresses and new insights emerge.*
