use std::collections::{BTreeMap, BTreeSet};

use crate::capability::capabilities_for_segment;
use crate::error::RvfError;
use crate::manifest::Manifest;
use crate::rvf_file::RvfFile;
use crate::segment::SegmentType;
use crate::traits::RvfAssembler;

/// Concrete builder that assembles an `RvfFile`.
///
/// Usage:
/// 1. `set_manifest(...)` — required
/// 2. `add_segment(...)` — one or more segments
/// 3. `build()` — produces the sealed `RvfFile`
#[derive(Debug, Default)]
pub struct RvfBuilder {
    manifest: Option<Manifest>,
    segments: BTreeMap<SegmentType, Vec<u8>>,
}

impl RvfBuilder {
    pub fn new() -> Self {
        Self::default()
    }
}

impl RvfAssembler for RvfBuilder {
    fn set_manifest(&mut self, manifest: Manifest) {
        self.manifest = Some(manifest);
    }

    fn add_segment(&mut self, seg_type: SegmentType, data: Vec<u8>) -> Result<(), RvfError> {
        if self.segments.contains_key(&seg_type) {
            return Err(RvfError::DuplicateSegment(seg_type.as_u8()));
        }
        self.segments.insert(seg_type, data);
        Ok(())
    }

    fn build(mut self) -> Result<RvfFile, RvfError> {
        // RF-1: MANIFEST_SEG must be present
        let mut manifest = self.manifest.take().ok_or(RvfError::ManifestNotSet)?;

        if !self.segments.contains_key(&SegmentType::ManifestSeg) {
            return Err(RvfError::MissingManifest);
        }

        // RF-2, RF-3, RF-4: Auto-populate capabilities from present segments
        let mut caps = BTreeSet::new();
        for &seg_type in self.segments.keys() {
            for &cap in capabilities_for_segment(seg_type) {
                caps.insert(cap);
            }
        }
        manifest.capabilities = caps.into_iter().collect();

        // RF-5: BTreeMap guarantees ordering by SegmentType discriminant
        // RF-6: file_hash computed inside RvfFile::new
        Ok(RvfFile::new(manifest, self.segments))
    }
}
