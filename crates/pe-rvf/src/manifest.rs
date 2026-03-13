use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::capability::Capability;
use crate::error::RvfError;

/// Root metadata for an RVF file.
///
/// Validated per RF-7: `parent_hash` and `signing_key_fingerprint`, if present,
/// must be exactly 32 bytes (SHA3-256 digest size).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub capabilities: Vec<Capability>,
    pub parent_hash: Option<[u8; 32]>,
    pub signing_key_fingerprint: Option<[u8; 32]>,
    pub created_at: DateTime<Utc>,
}

impl Manifest {
    /// Creates and validates a new manifest.
    pub fn new(
        name: String,
        version: String,
        parent_hash: Option<[u8; 32]>,
        signing_key_fingerprint: Option<[u8; 32]>,
        created_at: DateTime<Utc>,
    ) -> Result<Self, RvfError> {
        if name.is_empty() {
            return Err(RvfError::EmptyName);
        }
        if version.is_empty() {
            return Err(RvfError::EmptyVersion);
        }
        Ok(Self {
            name,
            version,
            capabilities: Vec::new(),
            parent_hash,
            signing_key_fingerprint,
            created_at,
        })
    }
}
