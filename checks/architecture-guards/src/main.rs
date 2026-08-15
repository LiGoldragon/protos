use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use syn::{Expr, ExprLit, Item, ItemStruct, Lit, Meta, Type, UseTree};

type Result<T> = std::result::Result<T, String>;

const FORBIDDEN_WORDS: [&str; 6] = ["archive", "code", "encode", "decode", "codec", "transcode"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Guard {
    FreeFunctions,
    InherentMethods,
    ZstBehavior,
    ForbiddenVocabulary,
}

impl Guard {
    const ALL: [Self; 4] = [
        Self::FreeFunctions,
        Self::InherentMethods,
        Self::ZstBehavior,
        Self::ForbiddenVocabulary,
    ];

    fn parse(value: &str) -> Result<Self> {
        match value {
            "free-functions" => Ok(Self::FreeFunctions),
            "inherent-methods" => Ok(Self::InherentMethods),
            "zst-behavior" => Ok(Self::ZstBehavior),
            "forbidden-vocabulary" => Ok(Self::ForbiddenVocabulary),
            _ => Err(format!("unknown guard `{value}`")),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::FreeFunctions => "free-functions",
            Self::InherentMethods => "inherent-methods",
            Self::ZstBehavior => "zst-behavior",
            Self::ForbiddenVocabulary => "forbidden-vocabulary",
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct TypeKey {
    module: Vec<String>,
    name: String,
}

#[derive(Clone, Debug)]
struct PathRef {
    segments: Vec<String>,
}

struct Resolver<'a> {
    corpus: &'a Corpus,
    zsts: BTreeSet<TypeKey>,
    aliases: BTreeMap<TypeKey, PathRef>,
    imports: BTreeMap<TypeKey, PathRef>,
    module_bindings: BTreeMap<TypeKey, PathRef>,
    globs: BTreeMap<Vec<String>, Vec<PathRef>>,
    modules: BTreeSet<Vec<String>>,
}

struct Unit {
    file: PathBuf,
    module: Vec<String>,
    items: Vec<Item>,
}

#[derive(Default)]
struct Corpus {
    units: Vec<Unit>,
    sources: BTreeMap<PathBuf, String>,
    loaded_files: HashSet<PathBuf>,
    loaded_modules: HashSet<(PathBuf, Vec<String>)>,
    active_files: HashSet<PathBuf>,
}

#[derive(Debug)]
struct Issue {
    file: PathBuf,
    detail: String,
}

impl Issue {
    fn new(file: &Path, detail: impl Into<String>) -> Self {
        Self {
            file: file.to_path_buf(),
            detail: detail.into(),
        }
    }
}

impl std::fmt::Display for Issue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.file.display(), self.detail)
    }
}

impl Corpus {
    fn from_paths(paths: Vec<PathBuf>) -> Result<Self> {
        let mut corpus = Self::default();
        for path in paths {
            corpus.add_file(path, Vec::new())?;
        }
        Ok(corpus)
    }

    fn production(root: &Path) -> Result<Self> {
        let root = root
            .canonicalize()
            .map_err(|error| format!("{}: {error}", root.display()))?;
        let lib = root.join("lib.rs");
        let mut corpus = Self::default();
        corpus.add_file(lib, Vec::new())?;

        // The module walk follows the compiled crate first. Any additional
        // Rust file is still included as its path-derived namespace so the
        // guard scans the whole source corpus without merging same-name types.
        let mut all_files = rust_files(&root)?;
        all_files.sort();
        for path in all_files {
            if !corpus.loaded_files.contains(&path) {
                let module = module_path_from_file(&root, &path);
                corpus.add_file(path, module)?;
            }
        }
        Ok(corpus)
    }

    fn add_file(&mut self, path: PathBuf, module: Vec<String>) -> Result<()> {
        let path = path
            .canonicalize()
            .map_err(|error| format!("{}: {error}", path.display()))?;
        if !self.loaded_modules.insert((path.clone(), module.clone())) {
            return Ok(());
        }
        self.loaded_files.insert(path.clone());
        if !self.active_files.insert(path.clone()) {
            return Ok(());
        }
        let source =
            fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let syntax =
            syn::parse_file(&source).map_err(|error| format!("{}: {error}", path.display()))?;
        self.sources.insert(path.clone(), source);
        let result = self.add_items(path.clone(), module, syntax.items);
        self.active_files.remove(&path);
        result
    }

    fn add_items(&mut self, file: PathBuf, module: Vec<String>, items: Vec<Item>) -> Result<()> {
        let children = module_children(&file, &module, &items)?;
        self.units.push(Unit {
            file: file.clone(),
            module: module.clone(),
            items: items.clone(),
        });
        for (child_module, child) in children {
            match child {
                ModuleChild::Inline(items) => {
                    self.add_items(file.clone(), child_module, items)?;
                }
                ModuleChild::External(path) => {
                    self.add_file(path, child_module)?;
                }
            }
        }
        Ok(())
    }
}

enum ModuleChild {
    Inline(Vec<Item>),
    External(PathBuf),
}

fn module_children(
    file: &Path,
    module: &[String],
    items: &[Item],
) -> Result<Vec<(Vec<String>, ModuleChild)>> {
    let mut children = Vec::new();
    for item in items {
        let Item::Mod(item_mod) = item else {
            continue;
        };
        let mut child_module = module.to_vec();
        child_module.push(ident_name(&item_mod.ident));
        if let Some((_, inline_items)) = &item_mod.content {
            children.push((child_module, ModuleChild::Inline(inline_items.clone())));
        } else {
            let path = external_module_file(file, item_mod)?;
            children.push((child_module, ModuleChild::External(path)));
        }
    }
    Ok(children)
}

fn external_module_file(parent: &Path, item_mod: &syn::ItemMod) -> Result<PathBuf> {
    let name = ident_name(&item_mod.ident);
    let parent_directory = parent
        .parent()
        .ok_or_else(|| format!("{} has no parent directory", parent.display()))?;
    if let Some(path) = module_path_attribute(item_mod) {
        let attributed = parent_directory.join(path);
        if attributed.is_file() {
            return Ok(attributed);
        }
        return Err(format!(
            "{} attributes module `{name}` with missing path",
            parent.display()
        ));
    }
    let parent_stem = parent.file_stem().and_then(|stem| stem.to_str());
    let directory = match parent_stem {
        Some("lib") | Some("main") | Some("mod") | None => parent_directory.to_path_buf(),
        Some(stem) => parent_directory.join(stem),
    };
    let flat = directory.join(format!("{name}.rs"));
    if flat.is_file() {
        return Ok(flat);
    }
    let nested = directory.join(&name).join("mod.rs");
    if nested.is_file() {
        return Ok(nested);
    }
    Err(format!(
        "{} declares missing module `{name}`",
        parent.display()
    ))
}

fn module_path_attribute(item_mod: &syn::ItemMod) -> Option<String> {
    item_mod.attrs.iter().find_map(|attribute| {
        if !attribute.path().is_ident("path") {
            return None;
        }
        let Meta::NameValue(name_value) = &attribute.meta else {
            return None;
        };
        let Expr::Lit(ExprLit {
            lit: Lit::Str(path),
            ..
        }) = &name_value.value
        else {
            return None;
        };
        Some(path.value())
    })
}

fn rust_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let entries = fs::read_dir(root).map_err(|error| format!("{}: {error}", root.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("{}: {error}", root.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("{}: {error}", path.display()))?;
        if file_type.is_dir() {
            files.extend(rust_files(&path)?);
        } else if file_type.is_file() && path.extension().is_some_and(|extension| extension == "rs")
        {
            files.push(path);
        }
    }
    Ok(files)
}

fn module_path_from_file(root: &Path, file: &Path) -> Vec<String> {
    let relative = file.strip_prefix(root).unwrap_or(file);
    let mut components = relative
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if let Some(last) = components.last_mut() {
        if last == "lib.rs" || last == "main.rs" {
            components.pop();
        } else if let Some(stem) = last.strip_suffix(".rs") {
            *last = stem.to_owned();
        }
    }
    if components
        .last()
        .is_some_and(|component| component == "mod")
    {
        components.pop();
    }
    components
}

fn ident_name(ident: &syn::Ident) -> String {
    let name = ident.to_string();
    name.strip_prefix("r#").unwrap_or(&name).to_owned()
}

fn check_guard(corpus: &Corpus, guard: Guard) -> Vec<Issue> {
    match guard {
        Guard::FreeFunctions => free_function_issues(corpus),
        Guard::InherentMethods => inherent_impl_issues(corpus),
        Guard::ZstBehavior => zst_issues(corpus),
        Guard::ForbiddenVocabulary => vocabulary_issues(corpus),
    }
}

fn free_function_issues(corpus: &Corpus) -> Vec<Issue> {
    corpus
        .units
        .iter()
        .flat_map(|unit| {
            unit.items.iter().filter_map(|item| {
                let Item::Fn(function) = item else {
                    return None;
                };
                Some(Issue::new(
                    &unit.file,
                    format!(
                        "free function `{}` in module `{}`",
                        ident_name(&function.sig.ident),
                        module_name(&unit.module)
                    ),
                ))
            })
        })
        .collect()
}

fn inherent_impl_issues(corpus: &Corpus) -> Vec<Issue> {
    corpus
        .units
        .iter()
        .flat_map(|unit| {
            unit.items.iter().filter_map(|item| {
                let Item::Impl(item_impl) = item else {
                    return None;
                };
                if item_impl.trait_.is_some() {
                    return None;
                }
                Some(Issue::new(
                    &unit.file,
                    format!("inherent impl in module `{}`", module_name(&unit.module)),
                ))
            })
        })
        .collect()
}

fn zst_issues(corpus: &Corpus) -> Vec<Issue> {
    let resolver = Resolver::new(corpus);
    corpus
        .units
        .iter()
        .flat_map(|unit| {
            unit.items.iter().filter_map(|item| {
                let Item::Impl(item_impl) = item else {
                    return None;
                };
                let key = resolver.resolve_self_type(&unit.module, &item_impl.self_ty)?;
                Some(Issue::new(
                    &unit.file,
                    format!("behavior attached to zero-sized `{}`", qualified_name(&key)),
                ))
            })
        })
        .collect()
}

fn is_zero_sized(item: &ItemStruct) -> bool {
    match &item.fields {
        syn::Fields::Unit => true,
        syn::Fields::Named(fields) => fields.named.is_empty(),
        syn::Fields::Unnamed(fields) => fields.unnamed.is_empty(),
    }
}

impl<'a> Resolver<'a> {
    fn new(corpus: &'a Corpus) -> Self {
        let modules = corpus
            .units
            .iter()
            .map(|unit| unit.module.clone())
            .collect::<BTreeSet<_>>();
        let zsts = corpus
            .units
            .iter()
            .flat_map(|unit| {
                unit.items.iter().filter_map(|item| {
                    let Item::Struct(item_struct) = item else {
                        return None;
                    };
                    is_zero_sized(item_struct).then(|| TypeKey {
                        module: unit.module.clone(),
                        name: ident_name(&item_struct.ident),
                    })
                })
            })
            .collect::<BTreeSet<_>>();
        let mut resolver = Self {
            corpus,
            zsts,
            aliases: BTreeMap::new(),
            imports: BTreeMap::new(),
            module_bindings: BTreeMap::new(),
            globs: BTreeMap::new(),
            modules,
        };

        for unit in &corpus.units {
            for item in &unit.items {
                match item {
                    Item::Type(item_type) if item_type.generics.params.is_empty() => {
                        if let Some(path) = type_path_ref(&item_type.ty) {
                            resolver.aliases.insert(
                                TypeKey {
                                    module: unit.module.clone(),
                                    name: ident_name(&item_type.ident),
                                },
                                path,
                            );
                        }
                    }
                    Item::Use(item_use) => {
                        let mut named = Vec::new();
                        let mut globs = Vec::new();
                        collect_use_tree(&item_use.tree, &mut Vec::new(), &mut named, &mut globs);
                        for (local, path) in named {
                            let key = TypeKey {
                                module: unit.module.clone(),
                                name: local,
                            };
                            resolver.imports.insert(key.clone(), path.clone());
                            resolver.module_bindings.insert(key, path);
                        }
                        resolver
                            .globs
                            .entry(unit.module.clone())
                            .or_default()
                            .extend(globs);
                    }
                    _ => {}
                }
            }
        }
        resolver
    }

    fn resolve_self_type(&self, module: &[String], ty: &Type) -> Option<TypeKey> {
        match ty {
            Type::Path(type_path) if type_path.qself.is_none() => {
                self.resolve_path(module, &path_ref(&type_path.path), &mut BTreeSet::new())
            }
            Type::Paren(paren) => self.resolve_self_type(module, &paren.elem),
            Type::Group(group) => self.resolve_self_type(module, &group.elem),
            _ => None,
        }
    }

    fn resolve_path(
        &self,
        module: &[String],
        path: &PathRef,
        visiting: &mut BTreeSet<TypeKey>,
    ) -> Option<TypeKey> {
        let first = path.segments.first()?.as_str();
        let start = match first {
            "crate" | "self" | "super" => 1,
            _ => 0,
        };
        let tail = &path.segments[start..];
        let name = tail.last()?.clone();
        let module_tail = &tail[..tail.len() - 1];
        if !module_tail.is_empty() {
            let target_module = self.resolve_module_path(
                module,
                &PathRef {
                    segments: path.segments[..path.segments.len() - 1].to_vec(),
                },
            )?;
            return self.resolve_binding(&target_module, &name, visiting);
        }

        let bases = self.relative_module_bases(module, first, path);
        for base in bases {
            if let Some(result) = self.resolve_binding(&base, &name, visiting) {
                return Some(result);
            }
        }
        None
    }

    fn resolve_binding(
        &self,
        module: &[String],
        name: &str,
        visiting: &mut BTreeSet<TypeKey>,
    ) -> Option<TypeKey> {
        let key = TypeKey {
            module: module.to_vec(),
            name: name.to_owned(),
        };
        if self.zsts.contains(&key) {
            return Some(key);
        }
        if !visiting.insert(key.clone()) {
            return None;
        }
        let target = self
            .aliases
            .get(&key)
            .or_else(|| self.imports.get(&key))
            .cloned();
        if let Some(target) = target {
            let result = self.resolve_path(module, &target, visiting);
            visiting.remove(&key);
            return result;
        }
        // A local item binding wins over a glob import, even when the local
        // item is data and therefore cannot itself resolve to a ZST.
        if self.local_item_names(module).contains(name) {
            visiting.remove(&key);
            return None;
        }
        let glob_paths = self.globs.get(module).cloned().unwrap_or_default();
        for glob in glob_paths {
            let Some(target_module) = self.resolve_module_path(module, &glob) else {
                continue;
            };
            if !self.public_names(&target_module, module).contains(name) {
                continue;
            }
            if let Some(result) = self.resolve_binding(&target_module, name, visiting) {
                visiting.remove(&key);
                return Some(result);
            }
        }
        visiting.remove(&key);
        None
    }

    fn resolve_module_path(&self, module: &[String], path: &PathRef) -> Option<Vec<String>> {
        self.resolve_module_path_inner(module, path, &mut BTreeSet::new())
    }

    fn resolve_module_path_inner(
        &self,
        module: &[String],
        path: &PathRef,
        visiting: &mut BTreeSet<TypeKey>,
    ) -> Option<Vec<String>> {
        let first = path.segments.first()?.as_str();
        let start = match first {
            "crate" | "self" | "super" => 1,
            _ => 0,
        };
        let bases = match first {
            "crate" => vec![Vec::new()],
            "self" => vec![module.to_vec()],
            "super" => {
                let mut base = module.to_vec();
                let mut index = 0;
                while path
                    .segments
                    .get(index)
                    .is_some_and(|segment| segment == "super")
                {
                    base.pop();
                    index += 1;
                }
                vec![base]
            }
            _ => (0..=module.len())
                .rev()
                .map(|prefix_len| module[..prefix_len].to_vec())
                .collect(),
        };
        let tail = &path.segments[start..];
        for base in bases {
            let mut candidate = base;
            let mut resolved = true;
            for segment in tail {
                let mut direct = candidate.clone();
                direct.push(segment.clone());
                if self.modules.contains(&direct) {
                    candidate = direct;
                    continue;
                }
                let key = TypeKey {
                    module: candidate.clone(),
                    name: segment.clone(),
                };
                let Some(alias) = self.module_bindings.get(&key).cloned() else {
                    resolved = false;
                    break;
                };
                if !visiting.insert(key.clone()) {
                    resolved = false;
                    break;
                }
                let Some(aliased) = self.resolve_module_path_inner(&candidate, &alias, visiting)
                else {
                    visiting.remove(&key);
                    resolved = false;
                    break;
                };
                visiting.remove(&key);
                candidate = aliased;
            }
            if resolved {
                return Some(candidate);
            }
        }
        None
    }

    fn relative_module_bases(
        &self,
        module: &[String],
        first: &str,
        path: &PathRef,
    ) -> Vec<Vec<String>> {
        match first {
            "crate" => vec![Vec::new()],
            "self" => vec![module.to_vec()],
            "super" => {
                let mut base = module.to_vec();
                let mut count = 0;
                while path
                    .segments
                    .get(count)
                    .is_some_and(|segment| segment == "super")
                {
                    base.pop();
                    count += 1;
                }
                vec![base]
            }
            _ => (0..=module.len())
                .rev()
                .map(|prefix_len| module[..prefix_len].to_vec())
                .collect(),
        }
    }

    fn public_names(&self, module: &[String], importer: &[String]) -> BTreeSet<String> {
        self.exported_names(module, importer, &mut BTreeSet::new())
    }

    fn exported_names(
        &self,
        module: &[String],
        importer: &[String],
        visiting: &mut BTreeSet<Vec<String>>,
    ) -> BTreeSet<String> {
        if !visiting.insert(module.to_vec()) {
            return BTreeSet::new();
        }
        let mut names = BTreeSet::new();
        for unit in self
            .corpus
            .units
            .iter()
            .filter(|unit| unit.module == module)
        {
            for item in &unit.items {
                match item {
                    Item::Struct(item_struct)
                        if is_visible_from(&item_struct.vis, module, importer) =>
                    {
                        names.insert(ident_name(&item_struct.ident));
                    }
                    Item::Type(item_type) if is_visible_from(&item_type.vis, module, importer) => {
                        names.insert(ident_name(&item_type.ident));
                    }
                    Item::Use(item_use) if is_visible_from(&item_use.vis, module, importer) => {
                        let mut named = Vec::new();
                        let mut globs = Vec::new();
                        collect_use_tree(&item_use.tree, &mut Vec::new(), &mut named, &mut globs);
                        names.extend(named.into_iter().map(|(name, _)| name));
                        for glob in globs {
                            if let Some(target) = self.resolve_module_path(module, &glob) {
                                names.extend(self.exported_names(&target, importer, visiting));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        visiting.remove(module);
        names
    }

    fn local_item_names(&self, module: &[String]) -> BTreeSet<String> {
        self.corpus
            .units
            .iter()
            .filter(|unit| unit.module == module)
            .flat_map(|unit| {
                unit.items.iter().filter_map(|item| match item {
                    Item::Enum(item) => Some(ident_name(&item.ident)),
                    Item::Struct(item) => Some(ident_name(&item.ident)),
                    Item::Trait(item) => Some(ident_name(&item.ident)),
                    Item::TraitAlias(item) => Some(ident_name(&item.ident)),
                    Item::Type(item) => Some(ident_name(&item.ident)),
                    Item::Union(item) => Some(ident_name(&item.ident)),
                    Item::Use(item) => {
                        let mut named = Vec::new();
                        let mut globs = Vec::new();
                        collect_use_tree(&item.tree, &mut Vec::new(), &mut named, &mut globs);
                        named.into_iter().map(|(name, _)| name).next()
                    }
                    _ => None,
                })
            })
            .collect()
    }
}

fn path_ref(path: &syn::Path) -> PathRef {
    PathRef {
        segments: path
            .segments
            .iter()
            .map(|segment| ident_name(&segment.ident))
            .collect(),
    }
}

fn type_path_ref(ty: &Type) -> Option<PathRef> {
    match ty {
        Type::Path(type_path) if type_path.qself.is_none() => Some(path_ref(&type_path.path)),
        Type::Paren(paren) => type_path_ref(&paren.elem),
        Type::Group(group) => type_path_ref(&group.elem),
        _ => None,
    }
}

fn collect_use_tree(
    tree: &UseTree,
    prefix: &mut Vec<String>,
    named: &mut Vec<(String, PathRef)>,
    globs: &mut Vec<PathRef>,
) {
    match tree {
        UseTree::Path(path) => {
            prefix.push(ident_name(&path.ident));
            collect_use_tree(&path.tree, prefix, named, globs);
            prefix.pop();
        }
        UseTree::Name(name) => {
            let mut path = prefix.clone();
            path.push(ident_name(&name.ident));
            named.push((ident_name(&name.ident), PathRef { segments: path }));
        }
        UseTree::Rename(rename) => {
            let path = if ident_name(&rename.ident) == "self" {
                if prefix.is_empty() {
                    vec!["self".to_owned()]
                } else {
                    prefix.clone()
                }
            } else {
                let mut path = prefix.clone();
                path.push(ident_name(&rename.ident));
                path
            };
            named.push((ident_name(&rename.rename), PathRef { segments: path }));
        }
        UseTree::Glob(_) => globs.push(PathRef {
            segments: prefix.clone(),
        }),
        UseTree::Group(group) => {
            for item in &group.items {
                collect_use_tree(item, prefix, named, globs);
            }
        }
    }
}

fn is_visible_from(
    visibility: &syn::Visibility,
    defined_module: &[String],
    importing_module: &[String],
) -> bool {
    match visibility {
        syn::Visibility::Public(_) => true,
        syn::Visibility::Inherited => importing_module.starts_with(defined_module),
        syn::Visibility::Restricted(restricted) => {
            let segments = restricted
                .path
                .segments
                .iter()
                .map(|segment| ident_name(&segment.ident))
                .collect::<Vec<_>>();
            let first = segments.first().map(String::as_str);
            let mut scope = match first {
                Some("crate") => Vec::new(),
                Some("self") => defined_module.to_vec(),
                Some("super") => {
                    let mut scope = defined_module.to_vec();
                    scope.pop();
                    scope
                }
                _ => return false,
            };
            let start = if matches!(first, Some("crate" | "self" | "super")) {
                1
            } else {
                0
            };
            scope.extend(segments.into_iter().skip(start));
            importing_module.starts_with(&scope)
        }
    }
}

fn vocabulary_issues(corpus: &Corpus) -> Vec<Issue> {
    corpus
        .sources
        .iter()
        .flat_map(|(file, source)| {
            vocabulary_matches(source)
                .into_iter()
                .map(|word| Issue::new(file, format!("forbidden vocabulary `{word}`")))
        })
        .collect()
}

fn vocabulary_matches(source: &str) -> Vec<String> {
    let chars = source.chars().collect::<Vec<_>>();
    let mut matches = Vec::new();
    for index in 0..chars.len() {
        for word in FORBIDDEN_WORDS {
            let word_chars = word.chars().collect::<Vec<_>>();
            if index + word_chars.len() > chars.len()
                || !chars[index..index + word_chars.len()]
                    .iter()
                    .zip(word_chars.iter())
                    .all(|(actual, expected)| actual.eq_ignore_ascii_case(expected))
            {
                continue;
            }
            let before_is_identifier = index
                .checked_sub(1)
                .and_then(|previous| chars.get(previous))
                .is_some_and(|character| unicode_ident::is_xid_continue(*character));
            let after_index = index + word_chars.len();
            let after_is_identifier = chars
                .get(after_index)
                .is_some_and(|character| unicode_ident::is_xid_continue(*character));
            if !before_is_identifier && !after_is_identifier {
                matches.push(word.to_owned());
            }
        }
    }
    matches
}

fn module_name(module: &[String]) -> String {
    if module.is_empty() {
        "crate".to_owned()
    } else {
        module.join("::")
    }
}

fn qualified_name(key: &TypeKey) -> String {
    if key.module.is_empty() {
        key.name.clone()
    } else {
        format!("{}::{}", key.module.join("::"), key.name)
    }
}

fn fixture_paths(root: &Path, guard: Guard, good: bool) -> Vec<PathBuf> {
    let names: &[&str] = match (guard, good) {
        (Guard::FreeFunctions, true) => &["free-functions-good.rs"],
        (Guard::FreeFunctions, false) => &["free-functions-bad.rs"],
        (Guard::InherentMethods, true) => &["inherent-methods-good.rs"],
        (Guard::InherentMethods, false) => &["inherent-methods-bad.rs"],
        (Guard::ZstBehavior, false) => &[
            "zst-bad.rs",
            "zst-cross-file-decl.rs",
            "zst-cross-file-impl.rs",
            "zst-alias-bad.rs",
        ],
        (Guard::ZstBehavior, true) => &["zst-good.rs", "zst-alias-good.rs"],
        (Guard::ForbiddenVocabulary, true) => &["vocabulary-good.rs"],
        (Guard::ForbiddenVocabulary, false) => &["vocabulary-bad.rs"],
    };
    names.iter().map(|name| root.join(name)).collect()
}

fn run_guard(production: &Corpus, fixtures: &Path, guard: Guard) -> Vec<String> {
    let mut failures = Vec::new();
    let bad = match Corpus::from_paths(fixture_paths(fixtures, guard, false)) {
        Ok(corpus) => corpus,
        Err(error) => {
            failures.push(format!(
                "{} fixtures could not be parsed: {error}",
                guard.name()
            ));
            return failures;
        }
    };
    let good = match Corpus::from_paths(fixture_paths(fixtures, guard, true)) {
        Ok(corpus) => corpus,
        Err(error) => {
            failures.push(format!(
                "{} fixtures could not be parsed: {error}",
                guard.name()
            ));
            return failures;
        }
    };
    let bad_issues = check_guard(&bad, guard);
    let minimum_bad_matches = match guard {
        Guard::ZstBehavior => 18,
        _ => 1,
    };
    if bad_issues.len() < minimum_bad_matches {
        failures.push(format!(
            "{} bad fixture reported {} matches; expected at least {}",
            guard.name(),
            bad_issues.len(),
            minimum_bad_matches
        ));
    }
    let good_issues = check_guard(&good, guard);
    failures.extend(
        good_issues
            .iter()
            .map(|issue| format!("{} good fixture was rejected: {issue}", guard.name())),
    );
    failures.extend(
        check_guard(production, guard)
            .iter()
            .map(|issue| format!("production/{}: {issue}", guard.name())),
    );
    failures
}

fn run(arguments: &[String]) -> Result<()> {
    if arguments.len() != 3 && arguments.len() != 5 {
        return Err(
            "usage: architecture-guards SOURCE_ROOT FIXTURE_ROOT [--guard NAME]".to_owned(),
        );
    }
    let selected = if arguments.len() == 5 {
        if arguments[3] != "--guard" {
            return Err("expected --guard".to_owned());
        }
        Some(Guard::parse(&arguments[4])?)
    } else {
        None
    };
    let production = Corpus::production(Path::new(&arguments[1]))?;
    let guards = selected.map_or_else(|| Guard::ALL.to_vec(), |guard| vec![guard]);
    let mut failures = Vec::new();
    for guard in guards {
        failures.extend(run_guard(&production, Path::new(&arguments[2]), guard));
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("\n"))
    }
}

fn main() {
    let arguments = env::args().collect::<Vec<_>>();
    if let Err(error) = run(&arguments) {
        eprintln!("architecture-guards: {error}");
        process::exit(1);
    }
}
