use pe_core::YamanakaFactor;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Filterable metadata stored alongside each embedding in the vector store.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariantMeta {
    pub variant_id: Uuid,
    pub target_factor: YamanakaFactor,
    pub generation: u32,
    pub composite_score: Option<f64>,
    pub design_method: DesignMethod,
}

/// How the variant was produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DesignMethod {
    WildType,
    Mutation,
    Crossover,
    Manual,
}
