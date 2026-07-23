//! The addressed structural table: the external sidecar keyed by `ScopedEncodedTypeId`.
//! Its content identity is computed over `TableIdentityPayload` and STORED OUTSIDE
//! that payload (fixing the self-reference bug), and is EXCLUDED from Core value
//! identity by construction — Core hashing never sees the table. Old table decodes
//! old text, a new table encodes new text, and both reach the same Core value (§4.6).

use std::collections::BTreeMap;

use content_identity::{ContentHash, DomainSeparation, HashDomain, LayoutVersion};

use crate::codec::StructuralEntry;
use crate::error::{DisjointnessError, TableError};
use crate::ids::{EncodedUniverseId, ScopedEncodedTypeId};

/// The identity of a Core layout the forms target (supplied by the Core side).
#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub struct EncodedLayoutIdentity(pub [u8; 32]);

/// The identity of a raw profile (glyph set + revision).
#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub struct RawProfileIdentity(pub [u8; 32]);

/// The identity of a leaf codec's contract.
#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub struct LeafCodecContractId(pub u32);

/// The table-identity pre-image. The resulting hash is stored on
/// `AddressedStructuralTable`, NEVER inside here.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct TableIdentityPayload {
    pub core_universe: EncodedUniverseId,
    pub core_layout_identity: EncodedLayoutIdentity,
    pub raw_profile_identity: RawProfileIdentity,
    pub leaf_codec_contracts: Vec<LeafCodecContractId>,
    pub entries: BTreeMap<ScopedEncodedTypeId, StructuralEntry>,
}

/// The hash domain for structural tables, layout-version tagged.
pub struct StructuralTableDomain;

impl HashDomain for StructuralTableDomain {
    fn separation() -> DomainSeparation {
        DomainSeparation::Contextual {
            context: "structural-codec 2026 addressed structural table",
            // Layout 4 admits typed delegation payloads and removes the unused
            // sigil representation from every archived form. The table identity
            // must move truthfully with that durable shape.
            layout: LayoutVersion::new(4),
        }
    }
}

/// A sealed structural table with its identity stored outside the hashed payload.
#[derive(Clone, Debug)]
pub struct AddressedStructuralTable {
    payload: TableIdentityPayload,
    identity: ContentHash<StructuralTableDomain>,
}

impl AddressedStructuralTable {
    /// Prove every decode form disjoint, then compute the table identity over its
    /// complete payload and store that identity outside its pre-image.
    pub fn seal(payload: TableIdentityPayload) -> Result<Self, TableError> {
        for entry in payload.entries.values() {
            entry.validate_disjoint_with(&payload.entries)?;
        }
        let identity = ContentHash::of_core(&payload)?;
        Ok(Self { payload, identity })
    }

    /// The table's content identity — co-versioned with the language package,
    /// EXCLUDED from Core value identity.
    pub fn identity(&self) -> ContentHash<StructuralTableDomain> {
        self.identity
    }

    /// Queried BY expected type, never globally searched; the input never selects its
    /// own type.
    pub fn entry(&self, expected: ScopedEncodedTypeId) -> Option<&StructuralEntry> {
        self.payload.entries.get(&expected)
    }

    /// Validate conservative disjointness across every entry.
    pub fn validate_disjoint(&self) -> Result<(), DisjointnessError> {
        for entry in self.payload.entries.values() {
            entry.validate_disjoint_with(&self.payload.entries)?;
        }
        Ok(())
    }
}
