//! ProteinVariant aggregate root.
//!
//! Represents an immutable protein variant in an evolutionary lineage.
//! New mutations or crossovers produce new variants rather than modifying existing ones.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::sequence::{AminoAcidSequence, CoreError, Mutation, YamanakaFactor};

/// Aggregate root representing a single protein variant in an evolutionary lineage.
///
/// All fields are private; access is via getter methods.
/// Variants are immutable after construction — mutations and crossovers produce new instances.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ProteinVariant {
    id: Uuid,
    name: String,
    sequence: AminoAcidSequence,
    target_factor: YamanakaFactor,
    mutations: Vec<Mutation>,
    generation: u32,
    parent_id: Option<Uuid>,
}

impl ProteinVariant {
    // ── Factory methods ──────────────────────────────────────────────

    /// Creates a generation-0 (wild-type) variant with no parent and no mutations.
    ///
    /// A new UUID v4 is generated automatically.
    pub fn wild_type(
        name: impl Into<String>,
        sequence: AminoAcidSequence,
        target_factor: YamanakaFactor,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            sequence,
            target_factor,
            mutations: Vec::new(),
            generation: 0,
            parent_id: None,
        }
    }

    /// Creates a child variant by applying a single mutation to a parent.
    ///
    /// # Errors
    ///
    /// * `CoreError::MutationPositionOutOfBounds` — mutation position >= parent sequence length (PV-5)
    /// * `CoreError::MutationFromResidueMismatch` — mutation `from_residue` does not match the
    ///   residue at that position in the parent sequence (PV-6)
    pub fn from_mutation(
        parent: &ProteinVariant,
        mutation: Mutation,
    ) -> Result<Self, CoreError> {
        let seq_len = parent.sequence.len();
        let pos = mutation.position();

        // PV-5: position must be within sequence bounds
        if pos >= seq_len {
            return Err(CoreError::MutationOutOfBounds {
                position: pos,
                length: seq_len,
            });
        }

        // PV-6: from_residue must match the actual residue at that position
        let actual = parent.sequence.as_slice()[pos];
        if mutation.from_residue() != actual {
            return Err(CoreError::MutationResidueMismatch {
                position: pos,
                expected: actual.to_char(),
                actual: mutation.from_residue().to_char(),
            });
        }

        // Build mutated sequence: replace residue at `pos` with `to_residue`
        let mut new_residues = parent.sequence.as_slice().to_vec();
        new_residues[pos] = mutation.to_residue();
        let new_sequence = AminoAcidSequence::from_residues(new_residues)?;

        // Accumulate parent mutations + this new one
        let mut mutations = parent.mutations.clone();
        mutations.push(mutation.clone());

        // Derive name: parent name + mutation shorthand (e.g., "+M123A")
        let mutation_label = format!(
            "+{}{}{}",
            mutation.from_residue().to_char(),
            pos,
            mutation.to_residue().to_char(),
        );
        let name = format!("{}{}", parent.name, mutation_label);

        Ok(Self {
            id: Uuid::new_v4(),
            name,
            sequence: new_sequence,
            target_factor: parent.target_factor,
            mutations,
            generation: parent.generation + 1,
            parent_id: Some(parent.id),
        })
    }

    /// Creates a child variant by crossing over two parent sequences.
    ///
    /// The new sequence is composed of `parent_a[0..crossover_point]` concatenated
    /// with `parent_b[crossover_point..]`.
    ///
    /// # Errors
    ///
    /// * `CoreError::CrossoverLengthMismatch` — parent sequences differ in length
    /// * `CoreError::CrossoverPointOutOfBounds` — crossover point is 0 or >= sequence length
    pub fn from_crossover(
        parent_a: &ProteinVariant,
        parent_b: &ProteinVariant,
        crossover_point: usize,
    ) -> Result<Self, CoreError> {
        let len_a = parent_a.sequence.len();
        let len_b = parent_b.sequence.len();

        // Sequences must be the same length
        if len_a != len_b {
            return Err(CoreError::CrossoverLengthMismatch {
                a: len_a,
                b: len_b,
            });
        }

        // crossover_point must be strictly between 0 and sequence length
        if crossover_point == 0 || crossover_point >= len_a {
            return Err(CoreError::CrossoverPointOutOfBounds {
                point: crossover_point,
                length: len_a,
            });
        }

        // Build new sequence: prefix from A, suffix from B
        let residues_a = parent_a.sequence.as_slice();
        let residues_b = parent_b.sequence.as_slice();

        let mut new_residues = Vec::with_capacity(len_a);
        new_residues.extend_from_slice(&residues_a[..crossover_point]);
        new_residues.extend_from_slice(&residues_b[crossover_point..]);

        let new_sequence = AminoAcidSequence::from_residues(new_residues)?;

        let generation = core::cmp::max(parent_a.generation, parent_b.generation) + 1;
        let name = format!("{}x{}@{}", parent_a.name, parent_b.name, crossover_point);

        Ok(Self {
            id: Uuid::new_v4(),
            name,
            sequence: new_sequence,
            target_factor: parent_a.target_factor,
            mutations: Vec::new(),
            generation,
            parent_id: Some(parent_a.id),
        })
    }

    // ── Getter methods ───────────────────────────────────────────────

    /// Returns the unique identifier for this variant.
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Returns the human-readable name/label.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns a reference to the amino acid sequence.
    pub fn sequence(&self) -> &AminoAcidSequence {
        &self.sequence
    }

    /// Returns a reference to the targeted Yamanaka factor.
    pub fn target_factor(&self) -> &YamanakaFactor {
        &self.target_factor
    }

    /// Returns a slice of all mutations relative to the wild-type ancestor.
    pub fn mutations(&self) -> &[Mutation] {
        &self.mutations
    }

    /// Returns the evolutionary generation number (0 for wild-type).
    pub fn generation(&self) -> u32 {
        self.generation
    }

    /// Returns the parent variant's ID, or `None` for wild-type.
    pub fn parent_id(&self) -> Option<Uuid> {
        self.parent_id
    }

    // ── Testing helpers ──────────────────────────────────────────────

    /// Sets a deterministic ID on this variant (builder pattern). Useful for tests.
    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = id;
        self
    }
}
