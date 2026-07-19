//! The composable interning identifier space.

use std::collections::{BTreeMap, HashMap};
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
            layout: LayoutVersion::new(2),
        }
    }
}

/// One namespace's owned names and transparent aliases.
///
/// The primary name at a local is the canonical re-emission name. Additional
/// names resolve to the same identifier during decode and are retained as
/// NameTree data so a textual projection can emit a transparent target-language
/// alias without adding an alias node to the EncodedForm.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct NameSlice {
    namespace: IdentifierNamespace,
    names: Vec<Name>,
    aliases: Vec<Vec<Name>>,
}

impl NameSlice {
    fn new(namespace: IdentifierNamespace) -> Self {
        Self {
            namespace,
            names: Vec::new(),
            aliases: Vec::new(),
        }
    }

    fn len(&self) -> usize {
        self.names.len()
    }

    fn name(&self, local: u16) -> Option<&Name> {
        self.names.get(usize::from(local))
    }

    fn aliases(&self, local: u16) -> Option<&[Name]> {
        self.aliases.get(usize::from(local)).map(Vec::as_slice)
    }

    fn identifiers(&self) -> impl Iterator<Item = (Name, Identifier)> + '_ {
        self.names
            .iter()
            .enumerate()
            .flat_map(move |(position, name)| {
                let local = u16::try_from(position).expect("name slice local exceeds u16 capacity");
                let identifier = self.namespace.identifier(local);
                std::iter::once((name.clone(), identifier)).chain(
                    self.aliases[position]
                        .iter()
                        .cloned()
                        .map(move |alias| (alias, identifier)),
                )
            })
    }

    fn intern(&mut self, name: Name) -> Identifier {
        let local = u16::try_from(self.names.len()).expect("name slice local exceeds u16 capacity");
        let identifier = self.namespace.identifier(local);
        self.names.push(name);
        self.aliases.push(Vec::new());
        identifier
    }

    fn add_alias(&mut self, local: u16, alias: Name) -> Result<(), NameTableError> {
        let aliases =
            self.aliases
                .get_mut(usize::from(local))
                .ok_or(NameTableError::UnknownIdentifier(
                    self.namespace.identifier(local),
                ))?;
        if !aliases.contains(&alias) {
            aliases.push(alias);
        }
        Ok(())
    }
}

/// An interned, composable map from [`Identifier`] to [`Name`].
///
/// A component owns exactly one home namespace and may borrow complete,
/// read-only slices from other components. Composition stores shared `Arc`
/// handles to those source slices; it does not call `extend_from`, copy source
/// names, or renumber their identifiers. `intern` always allocates in the home
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
        let home = Arc::new(NameSlice::new(namespace));
        Self::from_slices(home, BTreeMap::new())
    }

    fn from_slices(
        home: Arc<NameSlice>,
        borrowed: BTreeMap<IdentifierNamespace, Arc<NameSlice>>,
    ) -> Self {
        let mut table = Self {
            home,
            borrowed,
            index: HashMap::new(),
        };
        table.rebuild_index();
        table
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
        Ok(Self::from_slices(Arc::clone(&self.home), borrowed))
    }

    /// Intern a primary name into this component's home slice.
    ///
    /// A source name or transparent alias already present in any composed slice
    /// resolves to that existing identifier, so decoding carries literal names
    /// across component boundaries without reintroducing a string into Nomos.
    pub fn intern(&mut self, name: Name) -> Identifier {
        if let Some(identifier) = self.index.get(&name).copied() {
            return identifier;
        }

        let home = Arc::get_mut(&mut self.home).expect(
            "a NameTable home must finish allocation before another component borrows its slice",
        );
        let identifier = home.intern(name.clone());
        self.index.insert(name, identifier);
        identifier
    }

    /// Add another decoding name for an owned structural identifier.
    ///
    /// The alias is NameTree-only: it resolves to `target` during decode but does
    /// not create another encoded identifier or alter the EncodedForm graph.
    /// A textual projection can read [`Self::aliases`] to emit its transparent
    /// target-language alias declaration.
    pub fn add_alias(&mut self, target: Identifier, alias: Name) -> Result<(), NameTableError> {
        if target.namespace() != self.namespace() {
            return Err(NameTableError::BorrowedNamespace(target));
        }
        if let Some(existing) = self.index.get(&alias).copied() {
            if existing == target {
                return Ok(());
            }
            return Err(NameTableError::NameAlreadyAssigned {
                name: alias,
                existing,
            });
        }

        let home = Arc::get_mut(&mut self.home).expect(
            "a NameTable home must finish aliases before another component borrows its slice",
        );
        home.add_alias(target.local(), alias.clone())?;
        self.index.insert(alias, target);
        Ok(())
    }

    /// Resolve an identifier to its canonical primary name.
    pub fn resolve(&self, identifier: Identifier) -> Result<&Name, NameTableError> {
        self.slice(identifier.namespace())?
            .name(identifier.local())
            .ok_or(NameTableError::UnknownIdentifier(identifier))
    }

    /// The transparent aliases for an identifier, in declaration order.
    pub fn aliases(&self, identifier: Identifier) -> Result<&[Name], NameTableError> {
        self.slice(identifier.namespace())?
            .aliases(identifier.local())
            .ok_or(NameTableError::UnknownIdentifier(identifier))
    }

    /// Look up any canonical or transparent alias name without allocating.
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

    fn rebuild_index(&mut self) {
        self.index.clear();
        for (name, identifier) in self.home.identifiers() {
            self.index.insert(name, identifier);
        }
        for slice in self.borrowed.values() {
            for (name, identifier) in slice.identifiers() {
                self.index.entry(name).or_insert(identifier);
            }
        }
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
    ) -> Result<Value, Failure> {
        let mut transaction = self.begin();
        match attempt(&mut transaction) {
            Ok(value) => {
                transaction.commit();
                Ok(value)
            }
            Err(failure) => {
                transaction.rollback();
                Err(failure)
            }
        }
    }

    /// Merge a transaction's staged names into the home slice.
    pub(crate) fn commit_staged(&mut self, staged: Vec<Name>) {
        for name in staged {
            self.intern(name);
        }
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
        Ok(Self::from_slices(Arc::new(home), BTreeMap::new()))
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
    fn intern(&mut self, name: Name) -> Identifier {
        NameTable::intern(self, name)
    }
}
