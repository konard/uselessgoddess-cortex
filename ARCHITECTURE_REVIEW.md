# Architecture Review: The Cortex

## Summary

Reviewed: ARCHITECTURE.md for RAG-based LLM engine for AI-Dungeon style DnD5e roguelike.
Target: 16GB VRAM, quantized Qwen2.5-14B-Instruct.

**Verdict**: Feasible with modifications. Core concepts are sound. Several Rust patterns need refinement.

---

## 1. Hardware Feasibility

### VRAM Budget Analysis (16GB)

| Component | VRAM Estimate |
|-----------|---------------|
| Qwen2.5-14B Q4_K_M | ~8.5GB |
| KV cache (4k context) | ~1.5GB |
| KV cache (8k context) | ~3GB |
| Overhead/buffers | ~1GB |
| **Total (4k ctx)** | **~11GB** |
| **Total (8k ctx)** | **~13GB** |

**Assessment**: Workable. Use 4k-6k context windows. 8k possible but tight.

### Recommendations

- Set `num_ctx: 4096` in Ollama config as baseline
- Use GGUF Q4_K_M quantization (best quality/size ratio)
- Consider Q5_K_M if VRAM allows (~10GB model)
- Flash attention is mandatory (Ollama enables by default)
- Context window is your main constraint - the spreading activation approach addresses this well

---

## 2. Architectural Concerns

### 2.1 Entity ID Design

**Issue**: `EntityId(pub u32)` is exposed as public, uses `u32` inconsistently with `FactId(pub u32)`.

**Current**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(pub u32);
```

**Problem**: Mixing `u32` for entity IDs but `FactId` also uses `u32`. The `add_knowledge_fact` tool shows `FactId(Uuid::new_v4())` - contradicts the struct definition.

**Fix**:
```rust
use std::num::NonZeroU32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct EntityId(NonZeroU32);

impl EntityId {
    pub fn new(id: u32) -> Option<Self> {
        NonZeroU32::new(id).map(Self)
    }

    pub fn get(self) -> u32 {
        self.0.get()
    }
}
```

Benefits: niche optimization for `Option<EntityId>`, prevents zero IDs.

### 2.2 HashMap vs IndexMap

**Issue**: Using `HashMap<Tag, ...>` for knowledge graph traversal.

**Problem**: Non-deterministic iteration order affects spreading activation results between runs.

**Fix**:
```rust
use indexmap::IndexMap;

pub struct KnowledgeGraph {
    facts: IndexMap<FactId, Fact>,
    tag_to_facts: IndexMap<Tag, HashSet<FactId>>,
    associations: IndexMap<Tag, Vec<Association>>,
    // ...
}
```

### 2.3 Async Runtime Conflict

**Issue**: `NarrativeCore::process_event` uses `runtime.block_on()` inside a potentially async context.

```rust
impl NarrativeCore {
    pub fn process_event(&mut self, event: GameEvent, world_state: &WorldState) -> NarrativeResponse {
        self.runtime.block_on(
            self.processor.process_event(event, world_state)
        ).expect("Narrative processing failed")
    }
}
```

**Problem**: Will panic if called from async context. Bevy uses its own runtime.

**Fix**: Use channels or spawn blocking:
```rust
pub fn process_event(&self, event: GameEvent, world_state: &WorldState) -> impl Future<Output = NarrativeResponse> {
    let processor = self.processor.clone();
    let world_state = world_state.clone();
    async move {
        processor.process_event(event, &world_state).await
            .expect("Narrative processing failed")
    }
}
```

Or use `bevy_tokio_tasks` crate for proper integration.

### 2.4 Missing Error Types

**Issue**: `StorageError` is used but never defined.

**Fix**:
```rust
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Bincode(#[from] bincode::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
```

### 2.5 Tool Implementation Inconsistency

**Issue**: `ToolCall` in response types conflicts with `ToolCall` from Ollama response.

**Current** (two definitions):
```rust
// In llm_interface/response_types.rs
pub struct DialogueResponse {
    pub tool_calls: Vec<ToolCall>,  // But ToolCall not defined here
}

// In llm_interface/ollama.rs
pub struct ToolCall {
    pub function: FunctionCall,
}
```

**Fix**: Single definition, re-export:
```rust
// llm_interface/mod.rs
mod types;
mod ollama;

pub use types::*;
pub use ollama::ToolCall;  // Re-export
```

---

## 3. Rust Style Issues

### 3.1 Avoid `todo!()` in Production Code

**Issue**: `DiceExpr::parse` has `todo!()`.

**Fix**: Implement or return `Option`:
```rust
impl DiceExpr {
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        let (count, rest) = s.split_once('d')?;
        let count: u8 = count.parse().ok()?;

        let (sides, modifier) = if let Some((s, m)) = rest.split_once('+') {
            (s.parse().ok()?, m.parse::<i8>().ok()?)
        } else if let Some((s, m)) = rest.split_once('-') {
            (s.parse().ok()?, -m.parse::<i8>().ok()?)
        } else {
            (rest.parse().ok()?, 0)
        };

        Some(Self { count, sides, modifier })
    }
}
```

### 3.2 Unnecessary Cloning

**Issue**: `spread_activation` clones tags unnecessarily.

```rust
for (tag, energy) in hot_tags {
    // ...
    .map(|(t, e)| (t.clone(), e))
```

**Fix**: Use references where possible, or consider `Cow<'a, Tag>`.

### 3.3 Partial Comparisons with `unwrap()`

**Issue**:
```rust
tags.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
```

**Problem**: Panics on NaN.

**Fix**:
```rust
tags.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
// Or use total_cmp for f32 (Rust 1.62+):
tags.sort_by(|a, b| b.1.total_cmp(&a.1));
```

### 3.4 Use `entry` API Properly

**Issue**:
```rust
self.tag_to_facts
    .entry(tag.clone())
    .or_insert_with(HashSet::new)
    .insert(id);
```

**Fix**: Use `or_default()`:
```rust
self.tag_to_facts.entry(tag.clone()).or_default().insert(id);
```

### 3.5 Avoid String Allocation in Hot Paths

**Issue**: `Tag::as_string()` allocates every call.

**Fix**: Consider interning or returning `Cow`:
```rust
pub fn as_str(&self) -> Cow<'_, str> {
    match self {
        Tag::Entity(id) => Cow::Owned(format!("entity:{}", id.0)),
        Tag::Concept(s) => Cow::Borrowed(s.as_str()),
        // ...
    }
}
```

---

## 4. Missing Critical Components

### 4.1 No Prompt Token Counting

**Problem**: No mechanism to ensure prompts fit in context window.

**Fix**:
```rust
pub struct ContextBudget {
    max_tokens: usize,
    reserved_for_response: usize,
}

impl ContextAssembler {
    pub fn assemble_within_budget(
        &self,
        event: &GameEvent,
        graph: &KnowledgeGraph,
        world_state: &WorldState,
        budget: &ContextBudget,
    ) -> AssembledContext {
        // Prioritize facts by importance, truncate when budget exceeded
        // Use tiktoken-rs or approximate 4 chars = 1 token
    }
}
```

### 4.2 No Graceful Degradation

**Problem**: System fails completely if LLM is unavailable.

The `generate_fallback_response` exists but isn't integrated into the main flow.

**Fix**: Integrate fallback in `EventProcessor`:
```rust
pub async fn process_event(&mut self, event: GameEvent, world_state: &WorldState)
    -> Result<ProcessorOutput, ProcessingError>
{
    match self.try_process_with_llm(&event, world_state).await {
        Ok(output) => Ok(output),
        Err(ProcessingError::LLMError(_)) => {
            tracing::warn!("LLM unavailable, using fallback");
            Ok(generate_fallback_response(&event))
        }
        Err(e) => Err(e),
    }
}
```

### 4.3 No Schema Validation for LLM Output

**Problem**: `serde_json::from_str` will fail on malformed LLM output.

**Fix**: Use `schemars` + validation:
```rust
use schemars::schema_for;
use jsonschema::JSONSchema;

lazy_static! {
    static ref DIALOGUE_SCHEMA: JSONSchema = {
        let schema = schema_for!(DialogueResponse);
        JSONSchema::compile(&serde_json::to_value(schema).unwrap()).unwrap()
    };
}

fn parse_with_validation<T: DeserializeOwned + JsonSchema>(content: &str) -> Result<T, ParseError> {
    let value: Value = serde_json::from_str(content)?;
    // Validate before deserialize
    if let Err(errors) = DIALOGUE_SCHEMA.validate(&value) {
        return Err(ParseError::SchemaValidation(errors.collect()));
    }
    serde_json::from_value(value).map_err(Into::into)
}
```

---

## 5. Simplification Opportunities

### 5.1 Remove Redundant `FactType::Generic`

If facts can be generic, the type system isn't providing value. Consider:
```rust
pub struct Fact {
    // Always have structured type
    pub fact_type: FactType,  // No Generic variant
    // Or use optional specific fields
    pub relationship: Option<RelationshipData>,
    pub secret: Option<SecretData>,
}
```

### 5.2 Simplify Localization

If this is a single-developer project, `rust-i18n` adds complexity. Consider:
- Start with English only
- Use `&'static str` for built-in text
- Add i18n later via feature flag

### 5.3 Storage: Just Use JSON

Bincode is faster but:
- Not human-readable
- Version migration is painful
- Debugging is harder

For saves that are read once per session, JSON overhead is negligible.

---

## 6. Project Feasibility Assessment

### Realistic Scope for Solo Development

| Component | Complexity | Time Estimate |
|-----------|------------|---------------|
| `dnd_rules` types | Low | Weeks |
| Knowledge graph | Medium | Weeks |
| Context assembler | Medium | Weeks |
| Ollama integration | Low | Days |
| Tool system | Medium | Weeks |
| Bevy integration | High | Months |
| Content (bestiary, etc.) | High | Ongoing |

### Suggested MVP Path

1. **Phase 1**: Core loop
   - Hardcode world state
   - Simple prompt template
   - Ollama chat (no tools)
   - Text output only

2. **Phase 2**: Memory
   - Knowledge graph
   - Spreading activation
   - One tool (add_fact)

3. **Phase 3**: Structure
   - Typed responses
   - Full tool system
   - Bestiary loading

4. **Phase 4**: Game
   - Bevy integration
   - Combat system
   - Save/load

---

## 7. Critical Questions

1. **Combat AI latency**: 14B model needs ~2-5s per response. Is turn-based acceptable?

2. **Context window pressure**: With 4k tokens, you have ~2500 for context + ~1500 for response. Is that enough for complex scenarios?

3. **Dialogue branching**: How deep? Each branch multiplies complexity.

4. **Content pipeline**: Who writes the bestiary, items, initial facts? This is likely the biggest bottleneck.

---

## 8. Summary of Required Changes

### Must Fix (Correctness)
- [ ] Define `StorageError` type
- [ ] Fix `FactId` type inconsistency (u32 vs Uuid)
- [ ] Handle async/sync runtime boundary
- [ ] Add token budget limiting

### Should Fix (Robustness)
- [ ] Use `IndexMap` for deterministic iteration
- [ ] Add LLM output validation
- [ ] Integrate fallback responses
- [ ] Replace `todo!()` with implementation

### Consider (Polish)
- [ ] Use `NonZeroU32` for IDs
- [ ] Use `total_cmp` for f32 sorting
- [ ] Reduce cloning in hot paths
- [ ] Simplify localization for MVP
