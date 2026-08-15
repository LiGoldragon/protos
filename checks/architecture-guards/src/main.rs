use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use syn::{Item, ItemStruct, Type};

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
        if !self.loaded_files.insert(path.clone()) {
            return Ok(());
        }
        let source =
            fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let syntax =
            syn::parse_file(&source).map_err(|error| format!("{}: {error}", path.display()))?;
        self.sources.insert(path.clone(), source);
        self.add_items(path, module, syntax.items)
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
            let path = external_module_file(file, &item_mod.ident)?;
            children.push((child_module, ModuleChild::External(path)));
        }
    }
    Ok(children)
}

fn external_module_file(parent: &Path, ident: &syn::Ident) -> Result<PathBuf> {
    let name = ident_name(ident);
    let parent_directory = parent
        .parent()
        .ok_or_else(|| format!("{} has no parent directory", parent.display()))?;
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

    corpus
        .units
        .iter()
        .flat_map(|unit| {
            unit.items.iter().filter_map(|item| {
                let Item::Impl(item_impl) = item else {
                    return None;
                };
                let key = resolve_self_type(&unit.module, &item_impl.self_ty, &zsts)?;
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

fn resolve_self_type(module: &[String], ty: &Type, zsts: &BTreeSet<TypeKey>) -> Option<TypeKey> {
    match ty {
        Type::Path(type_path) if type_path.qself.is_none() => {
            resolve_path(module, &type_path.path, zsts)
        }
        Type::Paren(paren) => resolve_self_type(module, &paren.elem, zsts),
        Type::Group(group) => resolve_self_type(module, &group.elem, zsts),
        _ => None,
    }
}

fn resolve_path(module: &[String], path: &syn::Path, zsts: &BTreeSet<TypeKey>) -> Option<TypeKey> {
    let segments = path
        .segments
        .iter()
        .map(|segment| ident_name(&segment.ident))
        .collect::<Vec<_>>();
    let first = segments.first()?.as_str();
    let (base, tail) = match first {
        "crate" => (Vec::new(), &segments[1..]),
        "self" => (module.to_vec(), &segments[1..]),
        "super" => {
            let mut index = 0;
            let mut base = module.to_vec();
            while segments
                .get(index)
                .is_some_and(|segment| segment == "super")
            {
                base.pop();
                index += 1;
            }
            (base, &segments[index..])
        }
        _ => (module.to_vec(), segments.as_slice()),
    };
    let name = tail.last()?.clone();
    let module_tail = &tail[..tail.len() - 1];

    if first == "crate" || first == "self" || first == "super" {
        let mut candidate_module = base;
        candidate_module.extend(module_tail.iter().cloned());
        return zsts
            .get(&TypeKey {
                module: candidate_module,
                name,
            })
            .cloned();
    }

    // Relative paths first search the current module, then its ancestors.
    for prefix_len in (0..=module.len()).rev() {
        let mut candidate_module = module[..prefix_len].to_vec();
        candidate_module.extend(module_tail.iter().cloned());
        let candidate = TypeKey {
            module: candidate_module,
            name: name.clone(),
        };
        if zsts.contains(&candidate) {
            return Some(candidate);
        }
    }
    None
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
        (Guard::ZstBehavior, true) => &["zst-good.rs"],
        (Guard::ZstBehavior, false) => &[
            "zst-bad.rs",
            "zst-cross-file-decl.rs",
            "zst-cross-file-impl.rs",
        ],
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
    if bad_issues.is_empty() {
        failures.push(format!("{} bad fixture produced no witness", guard.name()));
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
