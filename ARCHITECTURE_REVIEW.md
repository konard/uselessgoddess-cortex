# Architecture Review: The Cortex

## Summary

Reviewed: ARCHITECTURE.md for RAG-based LLM engine for AI-Dungeon style DnD5e roguelike.
Target: RX 9070 XT (16GB VRAM), local LLM inference via llama.cpp/GGUF.

**Verdict**: Feasible. Core spreading activation RAG concept is solid. Ministral-3-14B is the recommended model.

---

## 1. Model Selection: Qwen3-14B vs Ministral-3-14B

### Benchmark Comparison (Late 2025)

| Benchmark | Ministral-3-14B | Qwen3-14B | Winner |
|-----------|-----------------|-----------|--------|
| AIME25 (Math Reasoning) | 85.0 | 73.7 | Ministral |
| LiveCodeBench (Coding/JSON) | 64.6 | 59.3 | Ministral |
| Arena Hard (Instruction Following) | 55.1 | 42.7 | Ministral |
| GPQA Diamond | 71.2 | 66.3 | Ministral |
| MATH Maj@1 | ~90.4 | ~85 | Ministral |

### Analysis

**Ministral-3-14B wins on all key metrics.** This is unusual—"creative" models typically sacrifice logical precision. The benchmark data suggests Mistral has successfully combined both capabilities.

**Why Ministral-3-14B for D&D DM Engine:**

1. **Reasoning + Creativity**: D&D requires both narrative prose AND complex logic (deciding what facts to save, resolving game mechanics, tracking state). Ministral excels at both.

2. **JSON/Structured Output**: Higher LiveCodeBench score indicates better structured output generation—critical for tool calls and fact extraction.

3. **Writing Style**: Mistral models are known for "literary/novelistic" prose. Qwen tends toward concise, logical output—acceptable but less immersive for storytelling.

4. **Thinking Mode**: Both models support chain-of-thought reasoning. Ministral-3-14B-Reasoning-2512 variant has extended thinking capabilities.

### VRAM Budget (Ministral-3-14B)

| Quantization | Model Size | KV Cache (16k ctx) | Total VRAM |
|--------------|------------|-------------------|------------|
| Q4_K_M | ~9GB | ~3GB | ~12-13GB |
| Q5_K_M | ~10.5GB | ~3GB | ~14-15GB |
| Q6_K | ~12GB | ~3GB | ~16GB (tight) |

**Recommendation: Q5_K_M quantization**
- Quality loss minimal (~3% vs FP16)
- Leaves ~1-2GB headroom for system overhead
- 16k context comfortable; 32k possible with Q4_K_M

---

## 2. Architecture Decision: Single vs Dual Model

### Option A: Single 14B Model (Everything)

One Ministral-3-14B (Q5_K_M) handles both narrative generation AND fact extraction.

**Approach 1: Single Pass**
```
User Action → [Ministral-14B] → Narrative + JSON extraction in one response
```

**Approach 2: Two-Pass**
```
User Action → [Ministral-14B] → Narrative → [Ministral-14B] → JSON extraction
```

**Pros:**
- Simpler architecture, one model to optimize
- Full context available for extraction decisions
- No VRAM fragmentation

**Cons:**
- Single-pass requires model to context-switch mid-generation
- Two-pass doubles latency (~4-10s total)

### Option B: Dual Model (14B + 3B)

Ministral-3-14B for narrative; Llama-3.2-3B or Ministral-3-3B for extraction.

```
User Action → [Ministral-14B] → Narrative
                    ↓
             [3B Model] → Extract facts from narrative + context
```

**VRAM Split:**
| Component | VRAM |
|-----------|------|
| Ministral-3-14B Q4_K_M | ~9GB |
| Llama-3.2-3B Q4_K_M | ~2GB |
| KV caches (both) | ~3-4GB |
| **Total** | ~14-15GB |

**Pros:**
- 3B model runs in parallel while 14B generates
- Lower latency for extraction (3B is fast)
- Specialized models for specialized tasks

**Cons:**
- VRAM pressure—context windows limited
- Two models to configure and prompt
- Context not shared (3B sees narrative output, not full state)

### Recommendation: Single Model, Two-Pass

**Rationale:**

1. **Context matters for extraction**: The 14B model needs full game state to decide what's "important enough" to save. A 3B model seeing only narrative output will miss context.

2. **VRAM headroom**: Q5_K_M fits with 16k context comfortably. Dual model forces Q4_K_M with reduced context.

3. **Latency is acceptable**: Turn-based gameplay tolerates 4-5s response time. Two-pass with 14B: ~2-3s narrative + ~1-2s extraction = ~4-5s total.

4. **Simpler debugging**: One model, one prompt engineering workflow.

**Implementation:**
```
Phase 1: Generate Narrative
- Input: Game state, retrieved facts, player action
- Output: Prose narrative for player

Phase 2: Extract/Analyze
- Input: Narrative + game state + extraction schema
- Output: JSON with facts to persist, state changes
- Use GBNF grammar to force valid JSON
```

---

## 3. Structured Outputs vs Model Intelligence

### The Question

> Does having a "smarter" model matter if I'm forcing JSON syntax via GBNF grammar?

### Answer: Yes, Intelligence Still Matters

**GBNF guarantees syntactic validity, not semantic correctness.**

Example prompt: "Extract important facts from this scene."

**Dumber model with GBNF:**
```json
{
  "facts": [
    {"content": "The tavern exists", "importance": 0.9},
    {"content": "There is a door", "importance": 0.8}
  ]
}
```
Valid JSON. Useless facts.

**Smarter model with GBNF:**
```json
{
  "facts": [
    {"content": "Merchant Aldric owes 500 gold to the Crown", "importance": 0.95},
    {"content": "Aldric's daughter is secretly a mage", "importance": 0.85}
  ]
}
```
Valid JSON. Narratively significant facts.

### What Intelligence Affects

| Aspect | Grammar Impact | Intelligence Impact |
|--------|---------------|---------------------|
| JSON syntax | Guaranteed by GBNF | N/A |
| Field names correct | Guaranteed by schema | N/A |
| Value types match | Guaranteed by schema | N/A |
| **Value content quality** | No impact | Critical |
| **Logical consistency** | No impact | Critical |
| **Relevance of decisions** | No impact | Critical |
| **Hallucination rate** | No impact | Higher IQ = less |

### GBNF Limitations

1. **Cannot force semantic coherence**: Grammar can require a string; can't require the string makes sense.

2. **Cannot prevent hallucination**: Model can confidently output incorrect facts in valid JSON.

3. **Cannot enforce logic**: "If X then Y" reasoning happens in the model, not the grammar.

### Recommendation

Use GBNF/structured outputs AND a high-reasoning model:

- **GBNF**: Eliminates malformed responses, simplifies parsing
- **High-reasoning model (Ministral-3-14B)**: Ensures extracted content is meaningful

The 85% AIME25 score matters because it indicates the model can reason about what's important, what's connected, and what to save—not just generate syntactically valid output.

---

## 4. Token Budget Analysis

### Fantasy RPG Scenario Requirements

| Context Component | Token Estimate |
|-------------------|----------------|
| System prompt + JSON schema | 400-600 |
| Current scene description | 150-250 |
| World state (time, weather, location) | 50-100 |
| Active characters (3-5 NPCs) | 300-500 |
| Retrieved facts (knowledge graph) | 800-1500 |
| Dialogue history (last 3 turns) | 400-700 |
| **Total Input** | **2100-3650** |
| Response (narrative + thinking) | 1000-2500 |
| **Total Context Window Needed** | **4000-6000** |

For complex scenes (combat, revelations, multi-NPC dialogue): **8k-12k tokens ideal.**

### VRAM Calculation

Ministral-3-14B Q5_K_M with 16k context on RX 9070 XT:

| Component | VRAM |
|-----------|------|
| Model weights (Q5_K_M) | ~10.5GB |
| KV cache (16k context) | ~3-3.5GB |
| Overhead/buffers | ~1GB |
| **Total** | **~14.5-15GB** |

**Verdict**: Fits with ~1GB headroom. Safe.

---

## 5. Dialogue Branching Design

### Architecture (from ARCHITECTURE.md)

State machine, not dialogue tree. AI generates 2-4 player options per turn; consequences stored as facts.

### AI-Controlled Termination

1. **Natural endings**: AI returns `ends_dialogue: true` on final variant when conversation concludes naturally
2. **Player escape**: Always include a "leave conversation" option
3. **Depth biasing**: At turn N-2, inject hint to wrap up; at turn N, force conclusion options

**Complexity**: O(n) per turn. No branch explosion because state lives in knowledge graph, not in conversation tree.

---

## 6. Content Pipeline

### Strategy: D&D SRD + Lazy AI Generation

**Hand-written (SRD foundation):**
- Bestiary stat blocks from D&D 5e SRD
- Item definitions
- Core world facts, major locations

**AI-generated (on first encounter):**
- Unique names for generic creatures
- Personality traits, descriptions
- Lore connections

**Data format:**
```toml
# bestiary/goblin.toml
[base]
template = "srd:goblin"

[generation]
name_style = "goblin"
generate_description = true
personality_count = 2
```

Lazy generation reduces startup time and spreads compute cost across gameplay.

---

## 7. Implementation Roadmap

### Phase 1: Proof of Concept (Core Loop)

**Goal**: Player action → AI narrative response

**Tasks:**
1. Set up llama.cpp with Ministral-3-14B Q5_K_M GGUF
2. Create basic prompt template with system instructions
3. Implement simple CLI: player types action, AI responds
4. Test narrative quality and response latency

**Deliverable**: Interactive text game, no persistence

**Validation**:
- Response latency <3s for narrative generation
- Prose quality subjectively acceptable
- VRAM usage stable under 15GB

---

### Phase 2: Structured Output & Fact Extraction

**Goal**: AI extracts and persists important facts

**Tasks:**
1. Define JSON schema for fact extraction
2. Implement GBNF grammar for extraction response
3. Two-pass generation: narrative → extraction
4. Test fact quality (relevance, consistency)

**Deliverable**: Facts extracted and logged per turn

**Validation**:
- 100% valid JSON output (GBNF guarantees syntax)
- >80% of extracted facts are narratively relevant
- Combined latency <6s (narrative + extraction)

---

### Phase 3: Knowledge Graph & RAG

**Goal**: Context-aware generation using spreading activation

**Tasks:**
1. Implement knowledge graph (IndexMap for determinism)
2. Implement spreading activation retrieval
3. Integrate retrieved facts into narrative prompt
4. Token budget management (prioritize by activation energy)

**Deliverable**: AI responses informed by accumulated world state

**Validation**:
- References past events appropriately
- No context overflow (token budget enforced)
- Deterministic behavior (same seed → same results)

---

### Phase 4: Game Mechanics & Combat

**Goal**: D&D 5e combat integration

**Tasks:**
1. Load bestiary from TOML (SRD creatures)
2. Implement turn-based combat loop
3. AI selects creature actions based on context
4. Combat narrative generation

**Deliverable**: Functional combat encounters

**Validation**:
- Combat follows D&D 5e SRD rules
- AI tactical decisions are reasonable
- Combat latency acceptable (~3-5s per turn)

---

### Phase 5: Bevy Integration & Persistence

**Goal**: Full game engine integration

**Tasks:**
1. Bevy ECS components for game entities
2. WorldContext snapshot → narrative_core
3. Save/load knowledge graph (JSON format)
4. Async LLM calls (channels or bevy_tokio_tasks)

**Deliverable**: Playable game prototype

---

## 8. Summary

### Model Selection

| Factor | Recommendation |
|--------|---------------|
| **Model** | Ministral-3-14B-Reasoning-2512 |
| **Quantization** | Q5_K_M (~10.5GB) |
| **Context window** | 16k tokens |
| **Architecture** | Single model, two-pass |

### Key Technical Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Model | Ministral-3-14B | Wins all benchmarks; literary prose + strong reasoning |
| Single vs Dual | Single model | Context matters for extraction; simpler debugging |
| GBNF usage | Yes | Guarantees syntax; model intelligence handles semantics |
| Quantization | Q5_K_M | Best quality/VRAM balance for 16GB |

### Why Ministral-3-14B Over Qwen3-14B

1. **+11.3 points on AIME25**: Better mathematical/logical reasoning
2. **+5.3 points on LiveCodeBench**: Better structured output generation
3. **+12.4 points on Arena Hard**: Better instruction following
4. **Literary style**: More immersive prose for D&D narratives
5. **Apache 2.0**: Fully open license

### Implementation Priority

1. Core loop (narrative generation)
2. Structured extraction (GBNF + two-pass)
3. Knowledge graph (spreading activation)
4. Combat (SRD mechanics)
5. Game integration (Bevy)

### Code Quality Notes

- Use `IndexMap` for deterministic graph traversal
- Use `total_cmp` for f32 sorting
- Define proper error types
- JSON storage for human-readable saves
