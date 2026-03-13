//! Foundational biological types: amino acids, sequences, mutations, and Yamanaka factors.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::ops::Deref;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// CoreError
// ---------------------------------------------------------------------------

/// Unified validation error type for all domain invariants in pe-core.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum CoreError {
    #[error("empty amino acid sequence")]
    EmptySequence,

    #[error("invalid amino acid residue: '{0}'")]
    InvalidResidue(char),

    #[error("mutation from_residue must differ from to_residue")]
    IdenticalMutationResidues,

    #[error("mutation position {position} out of bounds for sequence of length {length}")]
    MutationOutOfBounds { position: usize, length: usize },

    #[error("mutation from_residue '{expected}' does not match residue '{actual}' at position {position}")]
    MutationResidueMismatch {
        position: usize,
        expected: char,
        actual: char,
    },

    #[error("crossover requires sequences of equal length, got {a} and {b}")]
    CrossoverLengthMismatch { a: usize, b: usize },

    #[error("crossover point {point} out of bounds for sequence of length {length}")]
    CrossoverPointOutOfBounds { point: usize, length: usize },

    #[error("fitness score {field} = {value} is not in [0.0, 1.0]")]
    FitnessScoreOutOfRange { field: &'static str, value: f64 },

    #[error("fitness score is NaN for field '{field}'")]
    FitnessScoreNaN { field: &'static str },

    #[error("fitness weights must sum to 1.0, got {sum}")]
    WeightsSumInvalid { sum: f64 },

    #[error("experiment result measured_values must not be empty")]
    EmptyMeasuredValues,

    #[error("experiment result has NaN or infinite value for key '{key}'")]
    NonFiniteMeasuredValue { key: String },

    #[error("instrument_id must not be empty")]
    EmptyInstrumentId,

    #[error("fitness weight must not be negative")]
    NegativeWeight,
}

// ---------------------------------------------------------------------------
// AminoAcid
// ---------------------------------------------------------------------------

/// One of the 20 standard amino acid residues.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AminoAcid {
    Ala,
    Cys,
    Asp,
    Glu,
    Phe,
    Gly,
    His,
    Ile,
    Lys,
    Leu,
    Met,
    Asn,
    Pro,
    Gln,
    Arg,
    Ser,
    Thr,
    Val,
    Trp,
    Tyr,
}

impl AminoAcid {
    /// Parse a single-letter amino acid code (case-insensitive).
    pub fn from_char(c: char) -> Result<Self, CoreError> {
        match c.to_ascii_uppercase() {
            'A' => Ok(AminoAcid::Ala),
            'C' => Ok(AminoAcid::Cys),
            'D' => Ok(AminoAcid::Asp),
            'E' => Ok(AminoAcid::Glu),
            'F' => Ok(AminoAcid::Phe),
            'G' => Ok(AminoAcid::Gly),
            'H' => Ok(AminoAcid::His),
            'I' => Ok(AminoAcid::Ile),
            'K' => Ok(AminoAcid::Lys),
            'L' => Ok(AminoAcid::Leu),
            'M' => Ok(AminoAcid::Met),
            'N' => Ok(AminoAcid::Asn),
            'P' => Ok(AminoAcid::Pro),
            'Q' => Ok(AminoAcid::Gln),
            'R' => Ok(AminoAcid::Arg),
            'S' => Ok(AminoAcid::Ser),
            'T' => Ok(AminoAcid::Thr),
            'V' => Ok(AminoAcid::Val),
            'W' => Ok(AminoAcid::Trp),
            'Y' => Ok(AminoAcid::Tyr),
            _ => Err(CoreError::InvalidResidue(c)),
        }
    }

    /// Return the uppercase single-letter code for this amino acid.
    pub fn to_char(&self) -> char {
        match self {
            AminoAcid::Ala => 'A',
            AminoAcid::Cys => 'C',
            AminoAcid::Asp => 'D',
            AminoAcid::Glu => 'E',
            AminoAcid::Phe => 'F',
            AminoAcid::Gly => 'G',
            AminoAcid::His => 'H',
            AminoAcid::Ile => 'I',
            AminoAcid::Lys => 'K',
            AminoAcid::Leu => 'L',
            AminoAcid::Met => 'M',
            AminoAcid::Asn => 'N',
            AminoAcid::Pro => 'P',
            AminoAcid::Gln => 'Q',
            AminoAcid::Arg => 'R',
            AminoAcid::Ser => 'S',
            AminoAcid::Thr => 'T',
            AminoAcid::Val => 'V',
            AminoAcid::Trp => 'W',
            AminoAcid::Tyr => 'Y',
        }
    }
}

impl fmt::Display for AminoAcid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_char())
    }
}

// ---------------------------------------------------------------------------
// AminoAcidSequence
// ---------------------------------------------------------------------------

/// A validated, immutable wrapper around a non-empty vector of amino acid residues.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AminoAcidSequence(Vec<AminoAcid>);

impl AminoAcidSequence {
    /// Parse a string of single-letter amino acid codes into a validated sequence.
    ///
    /// Returns [`CoreError::EmptySequence`] if `seq_str` is empty.
    /// Returns [`CoreError::InvalidResidue`] if any character is not a valid amino acid code.
    pub fn new(seq_str: &str) -> Result<Self, CoreError> {
        if seq_str.is_empty() {
            return Err(CoreError::EmptySequence);
        }

        let mut residues = Vec::with_capacity(seq_str.len());
        for c in seq_str.chars() {
            residues.push(AminoAcid::from_char(c)?);
        }

        Ok(AminoAcidSequence(residues))
    }

    /// Returns the number of residues in the sequence.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the sequence contains no residues.
    ///
    /// Note: construction enforces non-emptiness, so this always returns `false`
    /// for sequences created via [`AminoAcidSequence::new`].
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the residues as a slice.
    pub fn as_slice(&self) -> &[AminoAcid] {
        &self.0
    }

    /// Create a sequence directly from a `Vec<AminoAcid>`.
    ///
    /// Returns [`CoreError::EmptySequence`] if `residues` is empty.
    pub fn from_residues(residues: Vec<AminoAcid>) -> Result<Self, CoreError> {
        if residues.is_empty() {
            return Err(CoreError::EmptySequence);
        }
        Ok(AminoAcidSequence(residues))
    }

    /// Returns the one-letter sequence string.
    #[allow(clippy::inherent_to_string_shadow_display)]
    pub fn to_string(&self) -> String {
        self.0.iter().map(|aa| aa.to_char()).collect()
    }

    /// Returns the residue at the given zero-based position, or `None` if out of bounds.
    pub fn residue_at(&self, pos: usize) -> Option<AminoAcid> {
        self.0.get(pos).copied()
    }
}

impl Deref for AminoAcidSequence {
    type Target = [AminoAcid];

    fn deref(&self) -> &[AminoAcid] {
        &self.0
    }
}

impl fmt::Display for AminoAcidSequence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for aa in &self.0 {
            write!(f, "{}", aa.to_char())?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Mutation
// ---------------------------------------------------------------------------

/// A single amino acid substitution at a specific position in a sequence.
///
/// Invariants enforced at construction:
/// - `from_residue` and `to_residue` must differ (PV-7).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Mutation {
    position: usize,
    from_residue: AminoAcid,
    to_residue: AminoAcid,
}

impl Mutation {
    /// Create a new mutation, validating that `from_residue` differs from `to_residue`.
    pub fn new(
        position: usize,
        from_residue: AminoAcid,
        to_residue: AminoAcid,
    ) -> Result<Self, CoreError> {
        if from_residue == to_residue {
            return Err(CoreError::IdenticalMutationResidues);
        }
        Ok(Mutation {
            position,
            from_residue,
            to_residue,
        })
    }

    /// The zero-based position in the sequence where the substitution occurs.
    pub fn position(&self) -> usize {
        self.position
    }

    /// The original residue expected at the mutation position.
    pub fn from_residue(&self) -> AminoAcid {
        self.from_residue
    }

    /// The new residue that will replace the original.
    pub fn to_residue(&self) -> AminoAcid {
        self.to_residue
    }

    /// Apply this mutation to the given sequence, returning a new sequence with
    /// the substitution applied.
    ///
    /// Validates:
    /// - Position is within bounds (PV-5).
    /// - The residue at `position` matches `from_residue` (PV-6).
    pub fn apply(&self, sequence: &AminoAcidSequence) -> Result<AminoAcidSequence, CoreError> {
        let seq_len = sequence.len();

        if self.position >= seq_len {
            return Err(CoreError::MutationOutOfBounds {
                position: self.position,
                length: seq_len,
            });
        }

        let actual = sequence.as_slice()[self.position];
        if actual != self.from_residue {
            return Err(CoreError::MutationResidueMismatch {
                position: self.position,
                expected: self.from_residue.to_char(),
                actual: actual.to_char(),
            });
        }

        let mut residues = sequence.as_slice().to_vec();
        residues[self.position] = self.to_residue;
        Ok(AminoAcidSequence(residues))
    }
}

// ---------------------------------------------------------------------------
// YamanakaFactor
// ---------------------------------------------------------------------------

/// The four Yamanaka transcription factors used in cellular reprogramming.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum YamanakaFactor {
    OCT4,
    SOX2,
    KLF4,
    CMYC,
}

impl fmt::Display for YamanakaFactor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            YamanakaFactor::OCT4 => "OCT4",
            YamanakaFactor::SOX2 => "SOX2",
            YamanakaFactor::KLF4 => "KLF4",
            YamanakaFactor::CMYC => "CMYC",
        };
        write!(f, "{}", name)
    }
}
