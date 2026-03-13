//! pe-neural: Neural fitness scoring ensemble for protein variants.
//!
//! Implements `FitnessPredictor` via an ensemble of Transformer, LSTM,
//! and N-BEATS sub-models, with model weights loadable from QUANT_SEG.

pub mod ensemble;
pub mod error;
pub mod scorers;
pub mod segment;
pub mod traits;

pub use ensemble::EnsemblePredictor;
pub use error::NeuralError;
pub use scorers::{LstmScorer, NBeatsScorer, TransformerScorer};
pub use segment::{ModelWeights, QuantSegProducer};
pub use traits::{FitnessPredictor, ModelLoader, SubModelScorer};

#[cfg(test)]
mod tests;
