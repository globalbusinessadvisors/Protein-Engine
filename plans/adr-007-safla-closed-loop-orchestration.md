# ADR-007: SAFLA Closed-Loop Agent Orchestration

**Status:** Accepted
**Date:** 2026-03-13
**Deciders:** Platform Architecture Team
**Relates to:** FR-05, FR-08, FR-10

---

## Context

Protein engineering is an iterative optimization problem: design variants, predict fitness, validate structure, screen for safety, synthesize in the lab, measure results, and feed measurements back into the next design round. This cycle must run autonomously with minimal human intervention, while remaining auditable and controllable.

The SAFLA (Self-Aware Feedback Loop Algorithm) pattern from the rUv ecosystem provides a closed-loop architecture. Synaptic-Mesh and ruv-swarm provide the agent substrate for distributing tasks.

## Decision

**We implement the SAFLA closed-loop pattern with six specialized ephemeral agents coordinated by a `SwarmCoordinator`.** Each agent implements the `SwarmAgent` trait and handles one phase of the design cycle. Agents are created per-cycle and retired based on performance metrics managed by pe-governance (daa).

### Design Cycle

```
  ┌─────────────────────────────────────────────────────┐
  │                  SAFLA CLOSED LOOP                   │
  │                                                      │
  │  ┌──────────┐    ┌──────────┐    ┌──────────────┐   │
  │  │ DESIGN   │───►│ SCORE    │───►│ VALIDATE     │   │
  │  │ sequence │    │ fitness  │    │ structure    │   │
  │  │ explorer │    │ scorer   │    │ validator    │   │
  │  └──────────┘    └──────────┘    └──────┬───────┘   │
  │       ▲                                  │          │
  │       │                                  ▼          │
  │  ┌──────────┐    ┌──────────┐    ┌──────────────┐   │
  │  │ LEARN    │◄───│ MEASURE  │◄───│ SCREEN       │   │
  │  │ update   │    │ lab data │    │ toxicity     │   │
  │  │ models   │    │ ingest   │    │ screener     │   │
  │  └──────────┘    └──────────┘    └──────────────┘   │
  │                                                      │
  │  ┌──────────────────────────────────────────────┐   │
  │  │ QUANTUM DISPATCH (parallel, async)            │   │
  │  │ VQE energy validation on promising candidates │   │
  │  └──────────────────────────────────────────────┘   │
  └─────────────────────────────────────────────────────┘
```

### Agent Roles

| Agent | Trait | Responsibility |
|-------|-------|---------------|
| `SequenceExplorer` | `SwarmAgent` | Evolutionary mutation + crossover to generate new variants |
| `FitnessScorerAgent` | `SwarmAgent` | Delegates to `FitnessPredictor` to score candidate variants |
| `StructuralValidator` | `SwarmAgent` | ESMFold / HNSW plausibility check on scored candidates |
| `ToxicityScreener` | `SwarmAgent` | Oncogenic risk classification; filters unsafe variants |
| `ExperimentDesigner` | `SwarmAgent` | Generates Opentrons/Hamilton lab protocols for top candidates |
| `QuantumDispatcher` | `SwarmAgent` | Routes VQE/QAOA jobs to quantum backends for energy validation |

### Cycle Execution

```
async fn run_design_cycle(config: CycleConfig) -> Result<CycleResult>:
    // 1. DESIGN — explore sequence space
    candidates = sequence_explorer.execute(ExploreTask { population, mutation_rate })

    // 2. SCORE — predict fitness for each candidate
    scored = fitness_scorer.execute(ScoreTask { candidates })

    // 3. VALIDATE — structural plausibility check
    validated = structural_validator.execute(ValidateTask { scored })

    // 4. SCREEN — filter oncogenic risks
    safe = toxicity_screener.execute(ScreenTask { validated })

    // 5. QUANTUM (parallel) — energy validation on top-N
    if config.quantum_enabled:
        quantum_dispatcher.execute(QuantumTask { top_n: safe.take(N) })

    // 6. LOG — commit results to ledger
    ledger.append_entry(CycleComplete { generation, promoted: safe })

    // 7. LEARN — update model weights if lab data available
    if new_experiment_results:
        update_models(new_experiment_results)

    // 8. PROMOTE — top candidates to HOT_SEG
    return CycleResult { promoted: safe, generation: config.generation + 1 }
```

## Governance (daa)

The pe-governance crate (backed by the daa framework) manages agent lifecycle:

- **Budget allocation**: Limits compute time per agent per cycle
- **Performance tracking**: Agents that consistently produce low-quality candidates are retired
- **Priority adjustment**: If lab results show a particular Yamanaka factor is under-explored, its exploration budget increases
- **Agent retirement**: The `LifecycleManager` trait determines when to replace underperforming agents

## Rationale

- **Autonomous operation**: Once configured, the cycle runs without human intervention
- **Auditable**: Every cycle step is logged to JOURNAL_SEG with ML-DSA signatures
- **Extensible**: Adding a new agent (e.g., `SolubilityPredictor`) requires implementing `SwarmAgent` and inserting it into the cycle
- **Testable**: London School mocks for each agent role verify cycle logic without real scoring/quantum/lab

## Consequences

### Positive
- Complete design-to-measurement pipeline in a single automated loop
- Each agent is independently testable and replaceable
- Governance prevents runaway compute or stuck cycles
- Lab integration (ExperimentDesigner) bridges computational and wet-lab work

### Negative
- Agent coordination adds latency compared to a monolithic pipeline
- Ephemeral agent creation/teardown has overhead per cycle
- Governance heuristics (when to retire an agent) require tuning
- Full cycle depends on all 6 agents + ledger + optionally quantum — many failure points
