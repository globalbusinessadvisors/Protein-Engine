#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use uuid::Uuid;

    use pe_core::YamanakaFactor;

    use crate::manager::{DaaConfig, DaaLifecycleManager};
    use crate::traits::LifecycleManager;
    use crate::types::{AgentMetrics, AgentRole, CycleConfig, CycleResult};

    // ── helpers ──────────────────────────────────────────────────────────

    fn default_manager() -> DaaLifecycleManager {
        DaaLifecycleManager::new(DaaConfig::default())
    }

    fn agent(quality: f64, cycles: u64) -> AgentMetrics {
        AgentMetrics {
            agent_id: Uuid::new_v4(),
            role: AgentRole::SequenceExplorer,
            cycles_completed: cycles,
            avg_quality_score: quality,
            avg_latency_ms: 50.0,
            error_count: 0,
        }
    }

    fn agent_with_role(quality: f64, cycles: u64, role: AgentRole) -> AgentMetrics {
        AgentMetrics {
            agent_id: Uuid::new_v4(),
            role,
            cycles_completed: cycles,
            avg_quality_score: quality,
            avg_latency_ms: 50.0,
            error_count: 0,
        }
    }

    // ── retirement tests ────────────────────────────────────────────────

    #[test]
    fn should_retire_returns_true_for_low_quality_past_min_cycles() {
        let mgr = default_manager();
        // quality 0.1 < threshold 0.3, cycles 10 > min 5
        let a = agent(0.1, 10);
        assert!(mgr.should_retire(&a));
    }

    #[test]
    fn should_retire_returns_false_for_new_agent_below_min_cycles() {
        let mgr = default_manager();
        // quality 0.1 (low), but only 2 cycles < min 5 → not retired
        let a = agent(0.1, 2);
        assert!(!mgr.should_retire(&a));
    }

    #[test]
    fn should_retire_returns_false_for_high_quality_agent() {
        let mgr = default_manager();
        // quality 0.8 > threshold 0.3, many cycles → not retired
        let a = agent(0.8, 100);
        assert!(!mgr.should_retire(&a));
    }

    #[test]
    fn should_retire_boundary_at_threshold() {
        let mgr = default_manager();
        // quality exactly at threshold → not retired (must be strictly less)
        let a = agent(0.3, 10);
        assert!(!mgr.should_retire(&a));
    }

    #[test]
    fn should_retire_boundary_at_min_cycles() {
        let mgr = default_manager();
        // quality below threshold, cycles exactly at min (5) → retired
        // (must complete >= min_cycles to be eligible)
        let a = agent(0.1, 5);
        assert!(mgr.should_retire(&a));
        // One less than min → not retired
        let b = agent(0.1, 4);
        assert!(!mgr.should_retire(&b));
    }

    // ── budget allocation tests ─────────────────────────────────────────

    #[test]
    fn allocate_budget_distributes_proportionally_to_quality() {
        let mgr = default_manager();
        let agents = vec![agent(0.8, 10), agent(0.2, 10)];
        let config = CycleConfig {
            total_compute_ms: 10_000,
            total_variants: 100,
            min_compute_ms_per_agent: 0,
            min_variants_per_agent: 0,
        };

        let alloc = mgr.allocate_budget(&agents, &config);

        let b0 = alloc.get(&agents[0].agent_id).unwrap();
        let b1 = alloc.get(&agents[1].agent_id).unwrap();

        // High-quality agent should get ~4x the budget of low-quality
        assert!(b0.max_compute_ms > b1.max_compute_ms);
        // Total should not exceed budget
        assert!(b0.max_compute_ms + b1.max_compute_ms <= config.total_compute_ms);
        // Rough proportional check: 0.8/(0.8+0.2) = 0.8 → 80% of 10000 = 8000
        assert!(b0.max_compute_ms >= 7000);
        assert!(b1.max_compute_ms <= 3000);
    }

    #[test]
    fn allocate_budget_gives_minimum_to_all_agents() {
        let mgr = default_manager();
        let agents = vec![agent(0.9, 10), agent(0.01, 10)];
        let config = CycleConfig {
            total_compute_ms: 10_000,
            total_variants: 100,
            min_compute_ms_per_agent: 1_000,
            min_variants_per_agent: 5,
        };

        let alloc = mgr.allocate_budget(&agents, &config);

        let b0 = alloc.get(&agents[0].agent_id).unwrap();
        let b1 = alloc.get(&agents[1].agent_id).unwrap();

        // Both agents get at least the floor
        assert!(b0.max_compute_ms >= config.min_compute_ms_per_agent);
        assert!(b1.max_compute_ms >= config.min_compute_ms_per_agent);
        assert!(b0.max_variants >= config.min_variants_per_agent);
        assert!(b1.max_variants >= config.min_variants_per_agent);
    }

    #[test]
    fn allocate_budget_handles_single_agent() {
        let mgr = default_manager();
        let agents = vec![agent(0.5, 3)];
        let config = CycleConfig::default();

        let alloc = mgr.allocate_budget(&agents, &config);

        let b = alloc.get(&agents[0].agent_id).unwrap();
        // Single agent gets the entire budget
        assert_eq!(b.max_compute_ms, config.total_compute_ms);
        assert_eq!(b.max_variants, config.total_variants);
    }

    #[test]
    fn allocate_budget_handles_empty_agents() {
        let mgr = default_manager();
        let agents: Vec<AgentMetrics> = vec![];
        let config = CycleConfig::default();

        let alloc = mgr.allocate_budget(&agents, &config);
        assert!(alloc.allocations.is_empty());
    }

    #[test]
    fn allocate_budget_all_zero_quality_gets_equal_share() {
        let mgr = default_manager();
        let agents = vec![agent(0.0, 10), agent(0.0, 10)];
        let config = CycleConfig {
            total_compute_ms: 10_000,
            total_variants: 100,
            min_compute_ms_per_agent: 0,
            min_variants_per_agent: 0,
        };

        let alloc = mgr.allocate_budget(&agents, &config);

        let b0 = alloc.get(&agents[0].agent_id).unwrap();
        let b1 = alloc.get(&agents[1].agent_id).unwrap();

        // Both clamped to 0.01, so equal share
        assert_eq!(b0.max_compute_ms, b1.max_compute_ms);
    }

    // ── priority adjustment tests ───────────────────────────────────────

    #[test]
    fn adjust_priorities_increases_boost_for_under_explored_factor() {
        let mut mgr = default_manager();

        // OCT4 has high coverage (10), KLF4 has low coverage (1)
        let mut factor_coverage = BTreeMap::new();
        factor_coverage.insert("OCT4".to_string(), 10);
        factor_coverage.insert("SOX2".to_string(), 8);
        factor_coverage.insert("KLF4".to_string(), 1);
        factor_coverage.insert("CMYC".to_string(), 5);

        let result = CycleResult {
            factor_coverage,
            best_fitness: 0.85,
            promoted_count: 3,
        };

        mgr.adjust_priorities(&result);

        // KLF4 (1/10 coverage ratio) should have a much higher boost than OCT4 (10/10)
        let oct4_boost = mgr.factor_boost(&YamanakaFactor::OCT4);
        let klf4_boost = mgr.factor_boost(&YamanakaFactor::KLF4);

        assert!(
            klf4_boost > oct4_boost,
            "KLF4 boost ({}) should be > OCT4 boost ({})",
            klf4_boost,
            oct4_boost
        );
        // OCT4 has max coverage → boost should be 1.0
        assert!((oct4_boost - 1.0).abs() < 1e-10);
        // KLF4 has 1/10 coverage → boost should be clamped at 3.0 (max)
        // because 1/(1/10) = 10 > 3.0 cap
        assert!((klf4_boost - 3.0).abs() < 1e-10);
    }

    #[test]
    fn adjust_priorities_equal_coverage_gives_equal_boosts() {
        let mut mgr = default_manager();

        let mut factor_coverage = BTreeMap::new();
        factor_coverage.insert("OCT4".to_string(), 5);
        factor_coverage.insert("SOX2".to_string(), 5);
        factor_coverage.insert("KLF4".to_string(), 5);
        factor_coverage.insert("CMYC".to_string(), 5);

        let result = CycleResult {
            factor_coverage,
            best_fitness: 0.7,
            promoted_count: 2,
        };

        mgr.adjust_priorities(&result);

        assert!((mgr.factor_boost(&YamanakaFactor::OCT4) - 1.0).abs() < 1e-10);
        assert!((mgr.factor_boost(&YamanakaFactor::SOX2) - 1.0).abs() < 1e-10);
        assert!((mgr.factor_boost(&YamanakaFactor::KLF4) - 1.0).abs() < 1e-10);
        assert!((mgr.factor_boost(&YamanakaFactor::CMYC) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn adjust_priorities_missing_factor_gets_max_boost() {
        let mut mgr = default_manager();

        // CMYC not in coverage → coverage_for returns 0, ratio 0/10 → clamped to 0.1 → boost 3.0
        let mut factor_coverage = BTreeMap::new();
        factor_coverage.insert("OCT4".to_string(), 10);
        factor_coverage.insert("SOX2".to_string(), 10);
        factor_coverage.insert("KLF4".to_string(), 10);
        // CMYC intentionally absent

        let result = CycleResult {
            factor_coverage,
            best_fitness: 0.9,
            promoted_count: 5,
        };

        mgr.adjust_priorities(&result);

        let cmyc_boost = mgr.factor_boost(&YamanakaFactor::CMYC);
        assert!(
            (cmyc_boost - 3.0).abs() < 1e-10,
            "CMYC boost should be max (3.0), got {}",
            cmyc_boost
        );
    }

    // ── AgentMetrics constructor ────────────────────────────────────────

    #[test]
    fn agent_metrics_new_initializes_zeros() {
        let id = Uuid::new_v4();
        let m = AgentMetrics::new(id, AgentRole::QuantumDispatcher);

        assert_eq!(m.agent_id, id);
        assert_eq!(m.role, AgentRole::QuantumDispatcher);
        assert_eq!(m.cycles_completed, 0);
        assert!((m.avg_quality_score - 0.0).abs() < 1e-10);
        assert_eq!(m.error_count, 0);
    }

    // ── BudgetAllocation default ────────────────────────────────────────

    #[test]
    fn budget_allocation_default_is_empty() {
        let alloc = crate::types::BudgetAllocation::default();
        assert!(alloc.allocations.is_empty());
    }

    // ── DaaConfig custom ────────────────────────────────────────────────

    #[test]
    fn custom_config_changes_retirement_behavior() {
        let config = DaaConfig {
            min_cycles_before_retirement: 20,
            retirement_quality_threshold: 0.5,
            exploration_priority: 2.0,
        };
        let mgr = DaaLifecycleManager::new(config);

        // quality 0.4 < 0.5, but only 10 cycles < 20 → not retired
        assert!(!mgr.should_retire(&agent(0.4, 10)));
        // quality 0.4 < 0.5, cycles 25 > 20 → retired
        assert!(mgr.should_retire(&agent(0.4, 25)));
    }

    // ── all 6 roles ─────────────────────────────────────────────────────

    #[test]
    fn all_six_roles_can_be_evaluated() {
        let mgr = default_manager();
        let roles = [
            AgentRole::SequenceExplorer,
            AgentRole::FitnessScorerAgent,
            AgentRole::StructuralValidator,
            AgentRole::ToxicityScreener,
            AgentRole::ExperimentDesigner,
            AgentRole::QuantumDispatcher,
        ];

        for role in &roles {
            let a = agent_with_role(0.5, 10, *role);
            // Should not retire a mid-quality agent
            assert!(!mgr.should_retire(&a));
        }
    }
}
