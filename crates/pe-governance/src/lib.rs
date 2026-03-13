//! pe-governance: Autonomous agent lifecycle management.
//!
//! Implements the `LifecycleManager` trait for agent retirement decisions,
//! budget allocation, and priority adjustment across SAFLA design cycles.

pub mod error;
pub mod manager;
pub mod traits;
pub mod types;

pub use error::GovernanceError;
pub use manager::{DaaConfig, DaaLifecycleManager};
pub use traits::LifecycleManager;
pub use types::{
    AgentMetrics, AgentRole, BudgetAllocation, BudgetEntry, CycleConfig, CycleResult,
};

#[cfg(test)]
mod tests;
