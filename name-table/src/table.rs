//! The composable interning identifier space.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use content_identity::{ContentHash, DomainSeparation, HashDomain, LayoutVersion, PortableArchive};

use crate::boundary::{NameInterner, NameResolver};
use crate::error::NameTableError;
use crate::identifier::{Identifier, IdentifierNamespace};
use crate::name::Name;
use crate::transaction::NameTransaction;

/// The hash domain of one namespace slice's content identity.
///
/// A slice has its own identity because a component borrows other slices instead
/// of copying them. The composed view's borrowed edges are runtime topology;
/// they are not duplicated into the home slice's stored content.
pub struct NameTableDomain;

impl HashDomain for NameTableDomain {
    fn separation() -> DomainSeparation {
        DomainSeparation::Contextual {
            context: "name-table 2026 sliced identifier space",
            layout: LayoutVersion::new(3),
        }
    }
}

/// One namespace's owned canonical names.
///
/// Each encoded identifier has exactly one canonical name in its component
/// projection. Additional source or target-language spellings are not part of
/// the language surface.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct NameSlice {
    namespace: IdentifierNamespace,
    names: Vec<Name>,
}

impl NameSlice {
    fn new(namespace: IdentifierNamespace) -> Self {
        Self {
            namespace,
            names: Vec::new(),
        }
    }

    fn len(&self) -> usize {
        self.names.len()
    }

    fn name(&self, local: u16) -> Option<&Name> {
        self.names.get(usize::from(local))
    }

    fn identifier_at(&self, position: usize) -> Result<Identifier, NameTableError> {
        let local = u16::try_from(position)
            .map_err(|_| NameTableError::NamespaceCapacity(self.namespace))?;
        Ok(self.namespace.identifier(local))
    }

    fn canonical_identifiers(&self) -> Result<Vec<(Name, Identifier)>, NameTableError> {
        self.validate()?;
        self.names
            .iter()
            .enumerate()
            .map(|(position, name)| Ok((name.clone(), self.identifier_at(position)?)))
            .collect()
    }

    fn validate(&self) -> Result<(), NameTableError> {
        if self.names.len() > usize::from(u16::MAX) + 1 {
            return Err(NameTableError::NamespaceCapacity(self.namespace));
        }

        let mut canonical_names = HashSet::with_capacity(self.names.len());
        for name in &self.names {
            if !canonical_names.insert(name) {
                return Err(NameTableError::DuplicateCanonicalName(name.clone()));
            }
        }
        Ok(())
    }

    fn intern(&mut self, name: Name) -> Result<Identifier, NameTableError> {
        let identifier = self.identifier_at(self.names.len())?;
        self.names.push(name);
        Ok(identifier)
    }
}

/// An interned, composable map from [`Identifier`] to [`Name`].
///
/// A component owns exactly one home namespace and may borrow complete,
/// read-only slices from other components. Composition stores shared `Arc`
/// handles to those source slices; it does not copy source names, flatten source
/// state, or renumber source identifiers. `intern` always allocates in the home
/// namespace, while `resolve` dispatches exhaustively by identifier variant.
///
/// A source component completes its own allocations before another component
/// borrows its slice. That lifecycle keeps the borrowed view immutable and makes
/// namespace-local identifiers stable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NameTable {
    home: Arc<NameSlice>,
    borrowed: BTreeMap<IdentifierNamespace, Arc<NameSlice>>,
    index: HashMap<Name, Identifier>,
}

impl NameTable {
    /// Create one component's empty NameTable with its owned allocation slice.
    pub fn new(namespace: IdentifierNamespace) -> Self {
        Self {
            home: Arc::new(NameSlice::new(namespace)),
            borrowed: BTreeMap::new(),
            index: HashMap::new(),
        }
    }

    fn from_slices(
        home: Arc<NameSlice>,
        borrowed: BTreeMap<IdentifierNamespace, Arc<NameSlice>>,
    ) -> Result<Self, NameTableError> {
        let mut table = Self {
            home,
            borrowed,
            index: HashMap::new(),
        };
        table.rebuild_index()?;
        Ok(table)
    }

    /// The namespace this component owns and appends to.
    pub fn namespace(&self) -> IdentifierNamespace {
        self.home.namespace
    }

    /// The number of names owned by this component's home slice.
    pub fn len(&self) -> usize {
        self.home.len()
    }

    /// Whether this component owns no names yet.
    pub fn is_empty(&self) -> bool {
        self.home.names.is_empty()
    }

    /// Borrow a completed slice into this component's one composed table.
    ///
    /// The source's names retain their original identifiers. Borrowing a second
    /// source with the same namespace is rejected, because an identifier variant
    /// has one authoritative slice.
    pub fn compose(&self, source: &NameTable) -> Result<Self, NameTableError> {
        let namespace = source.namespace();
        if namespace == self.namespace() || self.borrowed.contains_key(&namespace) {
            return Err(NameTableError::DuplicateNamespace(namespace));
        }

        let mut borrowed = self.borrowed.clone();
        borrowed.insert(namespace, Arc::clone(&source.home));
        Self::from_slices(Arc::clone(&self.home), borrowed)
    }

    /// Intern a canonical name into this component's home slice.
    ///
    /// A canonical source name already present in any composed slice resolves to
    /// that existing identifier without reintroducing a string into Nomos.
    pub fn intern(&mut self, name: Name) -> Result<Identifier, NameTableError> {
        if let Some(identifier) = self.index.get(&name).copied() {
            return Ok(identifier);
        }

        let home = Arc::get_mut(&mut self.home).ok_or(NameTableError::HomeSliceBorrowed {
            operation: "intern a name",
        })?;
        let identifier = home.intern(name.clone())?;
        self.index.insert(name, identifier);
        Ok(identifier)
    }

    /// Resolve an identifier to its canonical primary name.
    pub fn resolve(&self, identifier: Identifier) -> Result<&Name, NameTableError> {
        self.slice(identifier.namespace())?
            .name(identifier.local())
            .ok_or(NameTableError::UnknownIdentifier(identifier))
    }

    /// Look up a canonical name without allocating.
    pub fn lookup(&self, name: &Name) -> Option<Identifier> {
        self.index.get(name).copied()
    }

    fn slice(&self, namespace: IdentifierNamespace) -> Result<&NameSlice, NameTableError> {
        if namespace == self.namespace() {
            return Ok(self.home.as_ref());
        }
        self.borrowed
            .get(&namespace)
            .map(Arc::as_ref)
            .ok_or(NameTableError::UnknownNamespace(namespace))
    }

    fn rebuild_index(&mut self) -> Result<(), NameTableError> {
        let mut identifiers = self.home.canonical_identifiers()?;
        for slice in self.borrowed.values() {
            identifiers.extend(slice.canonical_identifiers()?);
        }

        self.index.clear();
        for (name, identifier) in identifiers {
            self.insert_indexed_name(name, identifier)?;
        }
        Ok(())
    }

    fn insert_indexed_name(
        &mut self,
        name: Name,
        identifier: Identifier,
    ) -> Result<(), NameTableError> {
        if let Some(existing) = self.index.insert(name.clone(), identifier) {
            return Err(NameTableError::NameIndexCollision {
                name,
                first: existing,
                second: identifier,
            });
        }
        Ok(())
    }

    /// Open a speculative transaction over this table. Names interned through the
    /// returned [`NameTransaction`] stage on the side; the committed table is not
    /// touched until [`NameTransaction::commit`]. Dropping the transaction (or
    /// calling [`NameTransaction::rollback`]) leaves this table byte-identical.
    pub fn begin(&mut self) -> NameTransaction<'_> {
        NameTransaction::new(self)
    }

    /// Run `attempt` against a speculative transaction, committing its interned
    /// names only if it succeeds. A failed alternative leaves no allocation
    /// effect: the table is byte-identical to before the call.
    pub fn try_intern<Value, Failure>(
        &mut self,
        attempt: impl FnOnce(&mut NameTransaction<'_>) -> Result<Value, Failure>,
    ) -> Result<Value, Failure>
    where
        Failure: From<NameTableError>,
    {
        let mut transaction = self.begin();
        match attempt(&mut transaction) {
            Ok(value) => {
                transaction.commit().map_err(Failure::from)?;
                Ok(value)
            }
            Err(failure) => {
                transaction.rollback();
                Err(failure)
            }
        }
    }

    /// Merge a transaction's staged names into the home slice.
    pub(crate) fn commit_staged(&mut self, staged: Vec<Name>) -> Result<(), NameTableError> {
        for name in staged {
            self.intern(name)?;
        }
        Ok(())
    }

    /// The home slice's canonical rkyv bytes. Borrowed slices are deliberately
    /// excluded: they remain independently content-identified and are composed
    /// again by their consumer rather than copied into this component's state.
    pub fn to_archive_bytes(&self) -> Result<rkyv::util::AlignedVec, NameTableError> {
        self.home
            .as_ref()
            .to_archive_bytes()
            .map_err(|error| NameTableError::Serialize(error.to_string()))
    }

    /// Reconstruct one uncomposed home slice from canonical archive bytes.
    /// Consumers compose required borrowed slices explicitly after loading.
    pub fn from_archive_bytes(bytes: &[u8]) -> Result<Self, NameTableError> {
        let home = NameSlice::from_archive_bytes(bytes)
            .map_err(|error| NameTableError::Deserialize(error.to_string()))?;
        home.validate()?;
        Self::from_slices(Arc::new(home), BTreeMap::new())
    }

    /// This home slice's content identity. Borrowed slices retain their own
    /// identities and are not folded into this component's owned name data.
    pub fn identity(&self) -> Result<ContentHash<NameTableDomain>, NameTableError> {
        ContentHash::<NameTableDomain>::of_core(self.home.as_ref())
            .map_err(|error| NameTableError::Serialize(error.to_string()))
    }
}

impl NameResolver for NameTable {
    fn resolve(&self, identifier: Identifier) -> Result<&Name, NameTableError> {
        NameTable::resolve(self, identifier)
    }
}

impl NameInterner for NameTable {
    fn intern(&mut self, name: Name) -> Result<Identifier, NameTableError> {
        NameTable::intern(self, name)
    }
}

#[cfg(test)]
mod archive_tests {
    use content_identity::PortableArchive;

    use super::{Name, NameSlice, NameTable};
    use crate::error::NameTableError;
    use crate::identifier::IdentifierNamespace;

    #[test]
    fn oversized_archived_slice_returns_a_typed_capacity_error() {
        let oversized = NameSlice {
            namespace: IdentifierNamespace::Schema,
            names: vec![Name::new("Repeated"); usize::from(u16::MAX) + 2],
        };
        let bytes = oversized
            .to_archive_bytes()
            .expect("archive oversized slice");

        assert!(matches!(
            NameTable::from_archive_bytes(bytes.as_ref()),
            Err(NameTableError::NamespaceCapacity(
                IdentifierNamespace::Schema
            ))
        ));
    }
}
