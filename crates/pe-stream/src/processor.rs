//! StreamProcessor — consumes an InstrumentSource and produces normalized ExperimentResults.

use pe_core::ExperimentResult;

use crate::error::StreamError;
use crate::normalizer::ReadingNormalizer;
use crate::traits::InstrumentSource;

/// Consumes raw instrument readings from a source and produces
/// normalized domain ExperimentResults.
pub struct StreamProcessor<S: InstrumentSource> {
    source: S,
    normalizer: ReadingNormalizer,
}

impl<S: InstrumentSource> StreamProcessor<S> {
    pub fn new(source: S, normalizer: ReadingNormalizer) -> Self {
        Self { source, normalizer }
    }

    /// Process the next reading from the source, returning a normalized ExperimentResult.
    ///
    /// Returns `Ok(None)` when the source is exhausted.
    pub async fn process_next(&mut self) -> Result<Option<ExperimentResult>, StreamError> {
        match self.source.read_next().await? {
            Some(reading) => {
                let result = self.normalizer.normalize(&reading)?;
                Ok(Some(result))
            }
            None => Ok(None),
        }
    }

    /// Process up to `max` readings from the source, returning all normalized results.
    pub async fn process_batch(
        &mut self,
        max: usize,
    ) -> Result<Vec<ExperimentResult>, StreamError> {
        let mut results = Vec::with_capacity(max);
        for _ in 0..max {
            match self.process_next().await? {
                Some(result) => results.push(result),
                None => break,
            }
        }
        Ok(results)
    }

    /// Access the underlying source.
    pub fn source(&self) -> &S {
        &self.source
    }
}
