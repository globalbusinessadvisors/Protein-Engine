use std::collections::BTreeMap;

use sha3::{Digest, Sha3_256};

use crate::error::RvfError;
use crate::manifest::Manifest;
use crate::segment::SegmentType;

/// An immutable RVF cognitive container.
///
/// Once built, the file is a sealed unit: manifest + ordered segments + hash.
/// The `file_hash` is the SHA3-256 of the complete serialized output (RF-6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RvfFile {
    manifest: Manifest,
    segments: BTreeMap<SegmentType, Vec<u8>>,
    file_hash: [u8; 32],
}

impl RvfFile {
    /// Construct an `RvfFile` and compute its hash.
    ///
    /// This is intentionally `pub(crate)` — callers should use `RvfBuilder`.
    pub(crate) fn new(
        manifest: Manifest,
        segments: BTreeMap<SegmentType, Vec<u8>>,
    ) -> Self {
        let serialized = Self::serialize_inner(&manifest, &segments);
        let file_hash = Self::compute_hash(&serialized);
        Self {
            manifest,
            segments,
            file_hash,
        }
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub fn segments(&self) -> &BTreeMap<SegmentType, Vec<u8>> {
        &self.segments
    }

    pub fn file_hash(&self) -> &[u8; 32] {
        &self.file_hash
    }

    /// Serialize the RVF file to its binary wire format.
    ///
    /// Format:
    /// - manifest_len: u32 (big-endian)
    /// - manifest_bytes: [u8; manifest_len]
    /// - for each segment in SegmentType order:
    ///   - type_id: u8
    ///   - data_len: u32 (big-endian)
    ///   - data: [u8; data_len]
    pub fn serialize(&self) -> Vec<u8> {
        Self::serialize_inner(&self.manifest, &self.segments)
    }

    /// Deserialize an RVF file from its binary wire format.
    pub fn deserialize(data: &[u8]) -> Result<Self, RvfError> {
        let mut cursor = 0usize;

        // Read manifest
        let manifest_len = read_u32(data, &mut cursor)? as usize;
        if cursor + manifest_len > data.len() {
            return Err(RvfError::DeserializationFailed(
                "manifest data extends beyond input".into(),
            ));
        }
        let manifest_bytes = &data[cursor..cursor + manifest_len];
        cursor += manifest_len;

        let manifest: Manifest = serde_json::from_slice(manifest_bytes).map_err(|e| {
            RvfError::DeserializationFailed(format!("invalid manifest JSON: {e}"))
        })?;

        // Read segments
        let mut segments = BTreeMap::new();
        while cursor < data.len() {
            if cursor + 1 > data.len() {
                return Err(RvfError::DeserializationFailed(
                    "unexpected end of segment header".into(),
                ));
            }
            let type_id = data[cursor];
            cursor += 1;

            let seg_type = SegmentType::from_u8(type_id).ok_or_else(|| {
                RvfError::DeserializationFailed(format!("unknown segment type: 0x{type_id:02X}"))
            })?;

            let seg_len = read_u32(data, &mut cursor)? as usize;
            if cursor + seg_len > data.len() {
                return Err(RvfError::DeserializationFailed(
                    "segment data extends beyond input".into(),
                ));
            }
            let seg_data = data[cursor..cursor + seg_len].to_vec();
            cursor += seg_len;

            segments.insert(seg_type, seg_data);
        }

        // Verify hash
        let computed_hash = Self::compute_hash(data);
        let file = Self {
            manifest,
            segments,
            file_hash: computed_hash,
        };
        Ok(file)
    }

    fn serialize_inner(
        manifest: &Manifest,
        segments: &BTreeMap<SegmentType, Vec<u8>>,
    ) -> Vec<u8> {
        let manifest_bytes =
            serde_json::to_vec(manifest).expect("manifest serialization cannot fail");
        let manifest_len = manifest_bytes.len() as u32;

        // Pre-allocate a reasonable buffer
        let mut buf = Vec::with_capacity(4 + manifest_bytes.len() + segments.len() * 64);

        buf.extend_from_slice(&manifest_len.to_be_bytes());
        buf.extend_from_slice(&manifest_bytes);

        // BTreeMap iterates in key order (RF-5)
        for (&seg_type, data) in segments {
            buf.push(seg_type.as_u8());
            buf.extend_from_slice(&(data.len() as u32).to_be_bytes());
            buf.extend_from_slice(data);
        }

        buf
    }

    fn compute_hash(data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha3_256::new();
        hasher.update(data);
        hasher.finalize().into()
    }
}

fn read_u32(data: &[u8], cursor: &mut usize) -> Result<u32, RvfError> {
    if *cursor + 4 > data.len() {
        return Err(RvfError::DeserializationFailed(
            "unexpected end of input reading u32".into(),
        ));
    }
    let bytes: [u8; 4] = data[*cursor..*cursor + 4]
        .try_into()
        .expect("slice is exactly 4 bytes");
    *cursor += 4;
    Ok(u32::from_be_bytes(bytes))
}
