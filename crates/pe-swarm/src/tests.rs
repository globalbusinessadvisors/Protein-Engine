#[cfg(test)]
mod tests {
    use pe_core::{
        AminoAcid, AminoAcidSequence, FitnessScore, FitnessWeights, ProteinVariant,
        ScoredVariant, YamanakaFactor,
    };
    use pe_rvf::traits::SegmentProducer;

    use crate::coordinator::DefaultCoordinator;
    use crate::evolution::SimpleEvolutionEngine;
    use crate::segment::HotSegProducer;
    use crate::traits::{EvolutionEngine, MockSwarmAgent, SwarmCoordinator};
    use crate::types::{AgentResult, AgentRole, AgentTask, CycleConfig};

    // Mock LedgerWriter locally since pe-ledger's MockLedgerWriter
    // is only available within pe-ledger's #[cfg(test)].
    mockall::mock! {
        pub Ledger {}
        impl pe_ledger::LedgerWriter for Ledger {
            fn append_entry(
                &mut self,
                entry_type: pe_ledger::EntryType,
                payload: Vec<u8>,
            ) -> Result<pe_ledger::EntryHash, pe_ledger::LedgerError>;
            fn verify_chain(&self) -> Result<bool, pe_ledger::LedgerError>;
            fn len(&self) -> usize;
        }
    }

    // ── helpers ──────────────────────────────────────────────────────────

    fn make_sequence(s: &str) -> AminoAcidSequence {
        AminoAcidSequence::new(s).unwrap()
    }

    fn make_variant(name: &str, seq: &str) -> ProteinVariant {
        ProteinVariant::wild_type(name, make_sequence(seq), YamanakaFactor::OCT4)
    }

    fn make_scored(name: &str, seq: &str, composite_approx: f64) -> ScoredVariant {
        let variant = make_variant(name, seq);
        let w = FitnessWeights::new(0.25, 0.25, 0.25, 0.25).unwrap();
        let score = FitnessScore::new(
            composite_approx.clamp(0.0, 1.0),
            composite_approx.clamp(0.0, 1.0),
            composite_approx.clamp(0.0, 1.0),
            0.0,
            &w,
        )
        .unwrap();
        ScoredVariant { variant, score }
    }

    fn mock_ledger() -> MockLedger {
        let mut ledger = MockLedger::new();
        ledger.expect_append_entry().returning(|_, _| {
            Ok(pe_ledger::EntryHash([0u8; 32]))
        });
        ledger.expect_len().returning(|| 0);
        ledger
    }

    // ── SAFLA closed loop test ──────────────────────────────────────────

    #[tokio::test]
    async fn safla_loop_runs_full_cycle_with_mock_agents() {
        // Explorer returns 10 variants
        let mut explorer = MockSwarmAgent::new();
        explorer.expect_role().return_const(AgentRole::SequenceExplorer);
        explorer.expect_execute().times(1).returning(|_task| {
            let variants: Vec<ProteinVariant> = (0..10)
                .map(|i| make_variant(&format!("v{}", i), "ACDEFGHIKLMNPQRSTVWY"))
                .collect();
            Box::pin(async move { Ok(AgentResult::Explored { variants }) })
        });

        // Scorer scores all 10
        let mut scorer = MockSwarmAgent::new();
        scorer.expect_role().return_const(AgentRole::FitnessScorer);
        scorer.expect_execute().times(1).returning(|task| {
            let candidates = match task {
                AgentTask::Score { candidates } => candidates,
                _ => panic!("expected Score task"),
            };
            let scored: Vec<ScoredVariant> = candidates
                .into_iter()
                .enumerate()
                .map(|(i, v)| {
                    let w = FitnessWeights::new(0.25, 0.25, 0.25, 0.25).unwrap();
                    let s = (i as f64 + 1.0) / 10.0;
                    let score = FitnessScore::new(s, s, s, 0.0, &w).unwrap();
                    ScoredVariant { variant: v, score }
                })
                .collect();
            Box::pin(async move { Ok(AgentResult::Scored { scored }) })
        });

        // Validator passes 8 of 10
        let mut validator = MockSwarmAgent::new();
        validator.expect_role().return_const(AgentRole::StructuralValidator);
        validator.expect_execute().times(1).returning(|task| {
            let scored = match task {
                AgentTask::Validate { scored } => scored,
                _ => panic!("expected Validate task"),
            };
            let passed = scored.into_iter().take(8).collect();
            Box::pin(async move { Ok(AgentResult::Validated { passed }) })
        });

        // Screener passes 7 of 8
        let mut screener = MockSwarmAgent::new();
        screener.expect_role().return_const(AgentRole::ToxicityScreener);
        screener.expect_execute().times(1).returning(|task| {
            let validated = match task {
                AgentTask::Screen { validated } => validated,
                _ => panic!("expected Screen task"),
            };
            let safe = validated.into_iter().take(7).collect();
            Box::pin(async move { Ok(AgentResult::Screened { safe }) })
        });

        let ledger = mock_ledger();

        let mut coord = DefaultCoordinator::new(
            Box::new(explorer),
            Box::new(scorer),
            Box::new(validator),
            Box::new(screener),
            None,
            Box::new(ledger),
        );

        let config = CycleConfig {
            generation: 1,
            population_size: 10,
            mutation_rate: 0.1,
            crossover_rate: 0.3,
            quantum_enabled: false,
            top_k: 100,
        };

        let result = coord.run_design_cycle(config).await.unwrap();

        assert_eq!(result.promoted.len(), 7);
        assert_eq!(result.variants_created, 10);
        assert_eq!(result.variants_scored, 10);
        assert_eq!(result.variants_validated, 8);
        assert_eq!(result.variants_screened, 7);
        assert_eq!(result.generation, 1);
    }

    // ── Quantum dispatch test ───────────────────────────────────────────

    #[tokio::test]
    async fn quantum_dispatch_called_when_enabled() {
        let mut explorer = MockSwarmAgent::new();
        explorer.expect_role().return_const(AgentRole::SequenceExplorer);
        explorer.expect_execute().returning(|_| {
            let variants = vec![make_variant("q1", "ACDEFGHIKLMNPQRSTVWY")];
            Box::pin(async move { Ok(AgentResult::Explored { variants }) })
        });

        let mut scorer = MockSwarmAgent::new();
        scorer.expect_role().return_const(AgentRole::FitnessScorer);
        scorer.expect_execute().returning(|task| {
            let candidates = match task {
                AgentTask::Score { candidates } => candidates,
                _ => panic!("expected Score task"),
            };
            let scored = candidates
                .into_iter()
                .map(|v| {
                    let w = FitnessWeights::new(0.25, 0.25, 0.25, 0.25).unwrap();
                    let score = FitnessScore::new(0.8, 0.8, 0.8, 0.1, &w).unwrap();
                    ScoredVariant { variant: v, score }
                })
                .collect();
            Box::pin(async move { Ok(AgentResult::Scored { scored }) })
        });

        let mut validator = MockSwarmAgent::new();
        validator.expect_role().return_const(AgentRole::StructuralValidator);
        validator.expect_execute().returning(|task| {
            let scored = match task {
                AgentTask::Validate { scored } => scored,
                _ => panic!("expected Validate"),
            };
            Box::pin(async move { Ok(AgentResult::Validated { passed: scored }) })
        });

        let mut screener = MockSwarmAgent::new();
        screener.expect_role().return_const(AgentRole::ToxicityScreener);
        screener.expect_execute().returning(|task| {
            let validated = match task {
                AgentTask::Screen { validated } => validated,
                _ => panic!("expected Screen"),
            };
            Box::pin(async move { Ok(AgentResult::Screened { safe: validated }) })
        });

        // Quantum agent — must be called exactly once
        let mut quantum = MockSwarmAgent::new();
        quantum.expect_role().return_const(AgentRole::QuantumDispatcher);
        quantum.expect_execute().times(1).returning(|_| {
            Box::pin(async move {
                Ok(AgentResult::QuantumDispatched { jobs_submitted: 1 })
            })
        });

        let ledger = mock_ledger();

        let mut coord = DefaultCoordinator::new(
            Box::new(explorer),
            Box::new(scorer),
            Box::new(validator),
            Box::new(screener),
            Some(Box::new(quantum)),
            Box::new(ledger),
        );

        let config = CycleConfig {
            generation: 0,
            population_size: 1,
            quantum_enabled: true,
            top_k: 10,
            ..Default::default()
        };

        let result = coord.run_design_cycle(config).await.unwrap();
        assert_eq!(result.promoted.len(), 1);
    }

    // ── Ledger integration test ─────────────────────────────────────────

    #[tokio::test]
    async fn ledger_append_called_with_cycle_completed() {
        let mut explorer = MockSwarmAgent::new();
        explorer.expect_role().return_const(AgentRole::SequenceExplorer);
        explorer.expect_execute().returning(|_| {
            let variants = vec![make_variant("l1", "ACDEFGHIKLMNPQRSTVWY")];
            Box::pin(async move { Ok(AgentResult::Explored { variants }) })
        });

        let mut scorer = MockSwarmAgent::new();
        scorer.expect_role().return_const(AgentRole::FitnessScorer);
        scorer.expect_execute().returning(|task| {
            let candidates = match task {
                AgentTask::Score { candidates } => candidates,
                _ => panic!("Score"),
            };
            let scored: Vec<_> = candidates
                .into_iter()
                .map(|v| {
                    let w = FitnessWeights::new(0.25, 0.25, 0.25, 0.25).unwrap();
                    ScoredVariant {
                        variant: v,
                        score: FitnessScore::new(0.5, 0.5, 0.5, 0.1, &w).unwrap(),
                    }
                })
                .collect();
            Box::pin(async move { Ok(AgentResult::Scored { scored }) })
        });

        let mut validator = MockSwarmAgent::new();
        validator.expect_role().return_const(AgentRole::StructuralValidator);
        validator.expect_execute().returning(|task| {
            let scored = match task {
                AgentTask::Validate { scored } => scored,
                _ => panic!("Validate"),
            };
            Box::pin(async move { Ok(AgentResult::Validated { passed: scored }) })
        });

        let mut screener = MockSwarmAgent::new();
        screener.expect_role().return_const(AgentRole::ToxicityScreener);
        screener.expect_execute().returning(|task| {
            let validated = match task {
                AgentTask::Screen { validated } => validated,
                _ => panic!("Screen"),
            };
            Box::pin(async move { Ok(AgentResult::Screened { safe: validated }) })
        });

        // Ledger — verify append is called with CycleCompleted
        let mut ledger = MockLedger::new();
        ledger
            .expect_append_entry()
            .withf(|entry_type, _payload| {
                *entry_type == pe_ledger::EntryType::CycleCompleted
            })
            .times(1)
            .returning(|_, _| Ok(pe_ledger::EntryHash([0u8; 32])));
        ledger.expect_len().returning(|| 1);

        let mut coord = DefaultCoordinator::new(
            Box::new(explorer),
            Box::new(scorer),
            Box::new(validator),
            Box::new(screener),
            None,
            Box::new(ledger),
        );

        let config = CycleConfig {
            generation: 5,
            population_size: 1,
            ..Default::default()
        };

        coord.run_design_cycle(config).await.unwrap();
    }

    // ── Empty population test ───────────────────────────────────────────

    #[tokio::test]
    async fn empty_population_returns_empty_result() {
        let mut explorer = MockSwarmAgent::new();
        explorer.expect_role().return_const(AgentRole::SequenceExplorer);

        let mut scorer = MockSwarmAgent::new();
        scorer.expect_role().return_const(AgentRole::FitnessScorer);

        let mut validator = MockSwarmAgent::new();
        validator.expect_role().return_const(AgentRole::StructuralValidator);

        let mut screener = MockSwarmAgent::new();
        screener.expect_role().return_const(AgentRole::ToxicityScreener);

        let ledger = MockLedger::new();

        let mut coord = DefaultCoordinator::new(
            Box::new(explorer),
            Box::new(scorer),
            Box::new(validator),
            Box::new(screener),
            None,
            Box::new(ledger),
        );

        let config = CycleConfig {
            generation: 0,
            population_size: 0,
            ..Default::default()
        };

        let result = coord.run_design_cycle(config).await.unwrap();
        assert!(result.promoted.is_empty());
        assert_eq!(result.variants_created, 0);
    }

    // ── Evolution engine tests ──────────────────────────────────────────

    #[test]
    fn mutate_produces_valid_child() {
        let engine = SimpleEvolutionEngine::new();
        let parent = make_variant("wt", "ACDEFGHIKLMNPQRSTVWY");

        let child = engine.mutate(&parent).unwrap();
        assert_eq!(child.generation(), parent.generation() + 1);
        assert_eq!(child.parent_id(), Some(parent.id()));
        assert_eq!(child.sequence().len(), parent.sequence().len());
        let parent_seq = parent.sequence().as_slice();
        let child_seq = child.sequence().as_slice();
        let diffs: usize = parent_seq
            .iter()
            .zip(child_seq.iter())
            .filter(|(a, b)| a != b)
            .count();
        assert_eq!(diffs, 1);
    }

    #[test]
    fn crossover_combines_parents() {
        let engine = SimpleEvolutionEngine::new();
        let a = make_variant("a", "AAAAAAAA");
        let b = make_variant("b", "CCCCCCCC");

        let child = engine.crossover(&a, &b).unwrap();
        let seq = child.sequence().as_slice();

        assert_eq!(seq.len(), 8);
        assert_eq!(seq[0], AminoAcid::Ala);
        assert_eq!(seq[7], AminoAcid::Cys);
    }

    #[test]
    fn select_returns_top_k_by_composite() {
        let engine = SimpleEvolutionEngine::new();
        let population = vec![
            make_scored("low", "ACDEFGHIKLMNPQRSTVWY", 0.2),
            make_scored("high", "ACDEFGHIKLMNPQRSTVWY", 0.9),
            make_scored("mid", "ACDEFGHIKLMNPQRSTVWY", 0.5),
        ];

        let selected = engine.select(&population, 2);
        assert_eq!(selected.len(), 2);
        assert!(selected[0].score.composite() > selected[1].score.composite());
    }

    #[test]
    fn select_with_top_k_greater_than_population() {
        let engine = SimpleEvolutionEngine::new();
        let population = vec![make_scored("a", "ACDEFGHIKLMNPQRSTVWY", 0.5)];

        let selected = engine.select(&population, 10);
        assert_eq!(selected.len(), 1);
    }

    // ── HOT_SEG segment test ────────────────────────────────────────────

    #[test]
    fn hot_seg_producer_serializes_candidates() {
        let candidates = vec![
            make_scored("h1", "ACDEFGHIKLMNPQRSTVWY", 0.9),
            make_scored("h2", "ACDEFGHIKLMNPQRSTVWY", 0.8),
        ];

        let producer = HotSegProducer::new(candidates);
        assert_eq!(producer.segment_type(), pe_rvf::SegmentType::HotSeg);

        let bytes = producer.produce().unwrap();
        let recovered: Vec<ScoredVariant> = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(recovered.len(), 2);
    }

    #[test]
    fn hot_seg_truncates_to_100() {
        let candidates: Vec<ScoredVariant> = (0..150)
            .map(|i| make_scored(&format!("v{}", i), "ACDEFGHIKLMNPQRSTVWY", 0.5))
            .collect();

        let producer = HotSegProducer::new(candidates);
        let bytes = producer.produce().unwrap();
        let recovered: Vec<ScoredVariant> = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(recovered.len(), 100);
    }

    // ── Agent role test ─────────────────────────────────────────────────

    #[test]
    fn agent_roles_are_distinct() {
        let roles = [
            AgentRole::SequenceExplorer,
            AgentRole::FitnessScorer,
            AgentRole::StructuralValidator,
            AgentRole::ToxicityScreener,
            AgentRole::ExperimentDesigner,
            AgentRole::QuantumDispatcher,
        ];

        for i in 0..roles.len() {
            for j in (i + 1)..roles.len() {
                assert_ne!(roles[i], roles[j]);
            }
        }
    }
}
