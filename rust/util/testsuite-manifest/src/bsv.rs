use regex::Regex;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;
use walkdir::WalkDir;

#[cfg(test)]
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct DependencyResolution {
    pub paths: BTreeSet<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct DependencyClosureResolution {
    pub paths: Vec<BTreeSet<String>>,
    pub data_paths: Vec<BTreeSet<String>>,
    pub foreign_link_paths: Vec<BTreeSet<String>>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct SourceReferences {
    imports: BTreeSet<String>,
    includes: BTreeSet<String>,
    foreign_modules: BTreeSet<String>,
    string_literals: BTreeSet<String>,
}

#[cfg(test)]
pub(crate) fn resolve_local_dependencies(
    fixture_root: &Path,
    roots: &BTreeSet<String>,
) -> DependencyResolution {
    let mut resolution =
        resolve_local_dependency_closures(fixture_root, std::slice::from_ref(roots));
    DependencyResolution {
        paths: resolution.paths.pop().unwrap_or_default(),
        diagnostics: resolution.diagnostics,
    }
}

pub(crate) fn resolve_local_dependency_closures(
    fixture_root: &Path,
    root_sets: &[BTreeSet<String>],
) -> DependencyClosureResolution {
    let mut diagnostics = Vec::new();
    let (files, packages) = index_fixture_directory(fixture_root, &mut diagnostics);
    let roots = root_sets
        .iter()
        .flat_map(BTreeSet::iter)
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut discovered = roots.clone();
    let mut queue = roots
        .iter()
        .filter(|path| files.contains(path.as_str()))
        .cloned()
        .collect::<VecDeque<_>>();
    let mut graph = BTreeMap::<String, BTreeSet<String>>::new();
    let mut runtime_data = BTreeSet::new();
    let mut foreign_link_files = BTreeSet::new();

    while let Some(path) = queue.pop_front() {
        if graph.contains_key(&path) || !is_bsv_source_or_include(&path) {
            continue;
        }
        let absolute = fixture_root.join(&path);
        let metadata = match fs::symlink_metadata(&absolute) {
            Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => metadata,
            Ok(_) => {
                diagnostics.push(format!(
                    "local BSV dependency {path:?} is not a regular non-symbolic-link file"
                ));
                graph.insert(path, BTreeSet::new());
                continue;
            }
            Err(error) => {
                diagnostics.push(format!(
                    "could not inspect local BSV dependency {path:?}: {error}"
                ));
                graph.insert(path, BTreeSet::new());
                continue;
            }
        };
        if metadata.len() == 0 {
            graph.insert(path, BTreeSet::new());
            continue;
        }
        let source = match fs::read(&absolute) {
            Ok(source) => source,
            Err(error) => {
                diagnostics.push(format!(
                    "could not read local BSV dependency {path:?}: {error}"
                ));
                graph.insert(path, BTreeSet::new());
                continue;
            }
        };
        let references = parse_references(&String::from_utf8_lossy(&source));
        let mut dependencies = BTreeSet::new();

        for package in references.imports {
            let Some(candidates) = packages.get(&package) else {
                // A package absent from the fixture directory is supplied by the BSC library path.
                continue;
            };
            if candidates.len() != 1 {
                diagnostics.push(format!(
                    "local BSV package {package:?} imported by {path:?} is ambiguous: {}",
                    candidates.iter().cloned().collect::<Vec<_>>().join(", ")
                ));
                continue;
            }
            dependencies.insert(
                candidates
                    .iter()
                    .next()
                    .expect("one package candidate")
                    .clone(),
            );
        }

        for include in references.includes {
            let Some(include_path) = resolve_include_path(&path, &include) else {
                diagnostics.push(format!(
                    "BSV include {include:?} in {path:?} escapes the fixture directory"
                ));
                continue;
            };
            if !files.contains(&include_path) {
                continue;
            }
            dependencies.insert(include_path);
        }

        for module in references.foreign_modules {
            let Some(foreign_path) = resolve_include_path(&path, &format!("{module}.v")) else {
                continue;
            };
            if files.contains(&foreign_path) {
                foreign_link_files.insert(foreign_path.clone());
                dependencies.insert(foreign_path);
            }
        }

        for literal in references.string_literals {
            for data_path in resolve_runtime_data_paths(&path, &literal, &files) {
                runtime_data.insert(data_path.clone());
                dependencies.insert(data_path);
            }
        }

        for dependency in &dependencies {
            if discovered.insert(dependency.clone()) {
                queue.push_back(dependency.clone());
            }
        }
        graph.insert(path, dependencies);
    }

    let paths = root_sets
        .iter()
        .map(|roots| {
            let mut closure = roots.clone();
            let mut queue = roots.iter().cloned().collect::<VecDeque<_>>();
            while let Some(path) = queue.pop_front() {
                if let Some(dependencies) = graph.get(&path) {
                    for dependency in dependencies {
                        if closure.insert(dependency.clone()) {
                            queue.push_back(dependency.clone());
                        }
                    }
                }
            }
            closure
        })
        .collect::<Vec<_>>();
    let data_paths = paths
        .iter()
        .map(|closure| closure.intersection(&runtime_data).cloned().collect())
        .collect();
    let foreign_link_paths = paths
        .iter()
        .map(|closure| closure.intersection(&foreign_link_files).cloned().collect())
        .collect();

    DependencyClosureResolution {
        paths,
        data_paths,
        foreign_link_paths,
        diagnostics,
    }
}

fn index_fixture_directory(
    fixture_root: &Path,
    diagnostics: &mut Vec<String>,
) -> (BTreeSet<String>, BTreeMap<String, BTreeSet<String>>) {
    let mut files = BTreeSet::new();
    let mut packages = BTreeMap::<String, BTreeSet<String>>::new();
    for entry in WalkDir::new(fixture_root).follow_links(false) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                diagnostics.push(format!(
                    "could not scan fixture directory for BSV dependencies: {error}"
                ));
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let Ok(relative) = entry.path().strip_prefix(fixture_root) else {
            diagnostics.push(format!(
                "fixture entry escaped its directory while indexing BSV dependencies: {}",
                entry.path().display()
            ));
            continue;
        };
        let relative = unix_path(relative);
        files.insert(relative.clone());
        if is_bsv_source(&relative) {
            if let Some(stem) = Path::new(&relative)
                .file_stem()
                .and_then(|stem| stem.to_str())
            {
                packages
                    .entry(stem.to_owned())
                    .or_default()
                    .insert(relative);
            }
        }
    }
    (files, packages)
}

fn parse_references(source: &str) -> SourceReferences {
    let source = mask_comments(source);
    SourceReferences {
        imports: import_regex()
            .captures_iter(&source)
            .map(|captures| captures[1].to_owned())
            .collect(),
        includes: include_regex()
            .captures_iter(&source)
            .map(|captures| captures[1].to_owned())
            .collect(),
        foreign_modules: foreign_module_regex()
            .captures_iter(&source)
            .map(|captures| captures[1].to_owned())
            .collect(),
        string_literals: string_literal_regex()
            .captures_iter(&source)
            .filter_map(|captures| {
                let literal = &captures[1];
                (!literal.contains('\\')).then(|| literal.to_owned())
            })
            .collect(),
    }
}

fn resolve_runtime_data_paths(
    source_path: &str,
    literal: &str,
    files: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    if literal.is_empty() || literal.contains(['\r', '\n']) {
        return paths;
    }
    if let Some(path) = resolve_include_path(source_path, literal) {
        if files.contains(&path) && !is_bsv_source_or_include(&path) {
            paths.insert(path);
        }
    }
    if !(literal.starts_with('.') || literal.starts_with('_')) || !literal.contains('.') {
        return paths;
    }
    let source_parent = Path::new(source_path)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    paths.extend(files.iter().filter_map(|path| {
        let candidate = Path::new(path);
        (candidate.parent().unwrap_or_else(|| Path::new("")) == source_parent
            && candidate
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(literal))
            && !is_bsv_source_or_include(path))
        .then(|| path.clone())
    }));
    paths
}

fn import_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?m)^[\t ]*import[\t ]+(?:qualified[\t ]+)?([A-Za-z_][A-Za-z0-9_]*)(?:[\t ]*::[^;\r\n]*;|[\t ]*;|[\t ]*(?:--[^\r\n]*)?\r?$)",
        )
        .expect("valid BSV/BH import regex")
    })
}

fn include_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"(?m)^[\t ]*`include[\t ]+"([^"\r\n]+)""#).expect("valid BSV include regex")
    })
}

fn foreign_module_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"(?m)^[\t ]*import[\t ]+"BVI"[\t ]+([A-Za-z_][A-Za-z0-9_$]*)[\t ]*="#)
            .expect("valid BVI foreign module regex")
    })
}

fn string_literal_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#""((?:\\.|[^"\\])*)""#).expect("valid BSV string literal regex")
    })
}

fn mask_comments(source: &str) -> String {
    #[derive(Clone, Copy)]
    enum State {
        Code,
        String,
        LineComment,
        BlockComment,
    }

    let bytes = source.as_bytes();
    let mut masked = bytes.to_vec();
    let mut state = State::Code;
    let mut index = 0;
    while index < bytes.len() {
        match state {
            State::Code if bytes[index..].starts_with(b"//") => {
                masked[index] = b' ';
                masked[index + 1] = b' ';
                state = State::LineComment;
                index += 2;
            }
            State::Code if bytes[index..].starts_with(b"/*") => {
                masked[index] = b' ';
                masked[index + 1] = b' ';
                state = State::BlockComment;
                index += 2;
            }
            State::Code if bytes[index] == b'"' => {
                state = State::String;
                index += 1;
            }
            State::Code => index += 1,
            State::String if bytes[index] == b'\\' && index + 1 < bytes.len() => index += 2,
            State::String if bytes[index] == b'"' => {
                state = State::Code;
                index += 1;
            }
            State::String => index += 1,
            State::LineComment if bytes[index] == b'\n' => {
                state = State::Code;
                index += 1;
            }
            State::LineComment => {
                masked[index] = b' ';
                index += 1;
            }
            State::BlockComment if bytes[index..].starts_with(b"*/") => {
                masked[index] = b' ';
                masked[index + 1] = b' ';
                state = State::Code;
                index += 2;
            }
            State::BlockComment => {
                if bytes[index] != b'\n' && bytes[index] != b'\r' {
                    masked[index] = b' ';
                }
                index += 1;
            }
        }
    }
    String::from_utf8(masked).expect("comment masking preserves UTF-8")
}

fn resolve_include_path(source_path: &str, include: &str) -> Option<String> {
    let include = include.replace('\\', "/");
    let include = Path::new(&include);
    if include.is_absolute() {
        return None;
    }
    let base = Path::new(source_path)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let mut components = Vec::<OsString>::new();
    for component in base.join(include).components() {
        match component {
            Component::Normal(component) => components.push(component.to_owned()),
            Component::CurDir => {}
            Component::ParentDir => {
                components.pop()?;
            }
            Component::Prefix(_) | Component::RootDir => return None,
        }
    }
    let path = components.into_iter().collect::<PathBuf>();
    (!path.as_os_str().is_empty()).then(|| unix_path(&path))
}

fn is_bsv_source(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension, "bsv" | "bs"))
}

fn is_bsv_source_or_include(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension, "bsv" | "bs" | "bsvh" | "h"))
}

fn unix_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    fn temporary_directory() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "bsc-bsv-dependencies-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn parses_static_imports_and_includes_but_ignores_comments() {
        let references = parse_references(
            r#"
                // import Commented::*;
                /*
                import AlsoCommented::*;
                */
                import Local::*;
                import ClassicSemicolon;
                import Legacy
                import qualified QualifiedLegacy -- trailing BH comment
                import "BVI" ForeignModule =
                `include "defs.bsvh"
            "#,
        );
        assert_eq!(
            references.imports,
            BTreeSet::from([
                "ClassicSemicolon".to_owned(),
                "Legacy".to_owned(),
                "Local".to_owned(),
                "QualifiedLegacy".to_owned(),
            ])
        );
        assert_eq!(
            references.includes,
            BTreeSet::from(["defs.bsvh".to_owned()])
        );
        assert_eq!(
            references.foreign_modules,
            BTreeSet::from(["ForeignModule".to_owned()])
        );
    }

    #[test]
    fn resolves_local_bvi_verilog_modules_as_foreign_link_paths() {
        let root = temporary_directory();
        fs::write(
            root.join("Main.bsv"),
            "import \"BVI\" Param =\nmodule mkParam(); endmodule\n",
        )
        .unwrap();
        fs::write(root.join("Param.v"), "module Param; endmodule\n").unwrap();
        fs::write(
            root.join("Missing.bsv"),
            "import \"BVI\" NotPresent =\nmodule mkMissing(); endmodule\n",
        )
        .unwrap();

        let resolution = resolve_local_dependency_closures(
            &root,
            &[
                BTreeSet::from(["Main.bsv".to_owned()]),
                BTreeSet::from(["Missing.bsv".to_owned()]),
            ],
        );
        let _ = fs::remove_dir_all(&root);

        assert!(resolution.diagnostics.is_empty());
        assert_eq!(
            resolution.paths,
            [
                BTreeSet::from(["Main.bsv".to_owned(), "Param.v".to_owned()]),
                BTreeSet::from(["Missing.bsv".to_owned()]),
            ]
        );
        assert_eq!(
            resolution.foreign_link_paths,
            [BTreeSet::from(["Param.v".to_owned()]), BTreeSet::new(),]
        );
    }

    #[test]
    fn resolves_recursive_local_imports_and_includes() {
        let root = temporary_directory();
        fs::write(
            root.join("Main.bsv"),
            "import Local::*;\nimport Prelude::*;\n",
        )
        .unwrap();
        fs::write(root.join("Local.bsv"), "`include \"defs.bsvh\"\n").unwrap();
        fs::write(root.join("defs.bsvh"), "// definitions\n").unwrap();

        let resolution =
            resolve_local_dependencies(&root, &BTreeSet::from(["Main.bsv".to_owned()]));
        let _ = fs::remove_dir_all(&root);

        assert!(resolution.diagnostics.is_empty());
        assert_eq!(
            resolution.paths,
            BTreeSet::from([
                "Main.bsv".to_owned(),
                "Local.bsv".to_owned(),
                "defs.bsvh".to_owned(),
            ])
        );
    }

    #[test]
    fn leaves_missing_includes_to_the_compiler() {
        let root = temporary_directory();
        fs::write(
            root.join("Main.bsv"),
            "`include \"intentionally_missing.bsv\"\n",
        )
        .unwrap();

        let resolution =
            resolve_local_dependencies(&root, &BTreeSet::from(["Main.bsv".to_owned()]));
        let _ = fs::remove_dir_all(&root);

        assert!(resolution.diagnostics.is_empty());
        assert_eq!(resolution.paths, BTreeSet::from(["Main.bsv".to_owned()]));
    }

    #[test]
    fn computes_independent_dependency_closures_from_one_index() {
        let root = temporary_directory();
        fs::write(root.join("One.bsv"), "import Shared::*;\n").unwrap();
        fs::write(root.join("Two.bsv"), "package Two; endpackage\n").unwrap();
        fs::write(root.join("Shared.bsv"), "`include \"defs.bsvh\"\n").unwrap();
        fs::write(root.join("defs.bsvh"), "// definitions\n").unwrap();

        let resolution = resolve_local_dependency_closures(
            &root,
            &[
                BTreeSet::from(["One.bsv".to_owned()]),
                BTreeSet::from(["Two.bsv".to_owned()]),
            ],
        );
        let _ = fs::remove_dir_all(&root);

        assert!(resolution.diagnostics.is_empty());
        assert_eq!(
            resolution.paths,
            [
                BTreeSet::from([
                    "One.bsv".to_owned(),
                    "Shared.bsv".to_owned(),
                    "defs.bsvh".to_owned(),
                ]),
                BTreeSet::from(["Two.bsv".to_owned()]),
            ]
        );
    }

    #[test]
    fn resolves_exact_and_composed_runtime_data_references() {
        let root = temporary_directory();
        fs::write(
            root.join("Main.bsv"),
            r#"
                let exact <- mkRegFileLoad("mem.data", 0, 15);
                let dynamic <- mkRegFileFullLoad(name + "_file.txt");
            "#,
        )
        .unwrap();
        fs::write(root.join("mem.data"), "00\n").unwrap();
        fs::write(root.join("first_file.txt"), "01\n").unwrap();
        fs::write(root.join("second_file.txt"), "02\n").unwrap();
        fs::write(root.join("unrelated.txt"), "03\n").unwrap();

        let resolution =
            resolve_local_dependency_closures(&root, &[BTreeSet::from(["Main.bsv".to_owned()])]);
        let _ = fs::remove_dir_all(&root);

        assert!(resolution.diagnostics.is_empty());
        assert_eq!(
            resolution.data_paths,
            [BTreeSet::from([
                "first_file.txt".to_owned(),
                "mem.data".to_owned(),
                "second_file.txt".to_owned(),
            ])]
        );
        assert_eq!(
            resolution.paths,
            [BTreeSet::from([
                "Main.bsv".to_owned(),
                "first_file.txt".to_owned(),
                "mem.data".to_owned(),
                "second_file.txt".to_owned(),
            ])]
        );
    }

    #[test]
    fn resolves_recursive_legacy_bh_imports() {
        let root = temporary_directory();
        fs::write(root.join("Main.bs"), "import Local\n").unwrap();
        fs::write(root.join("Local.bs"), "import Nested -- local package\n").unwrap();
        fs::write(root.join("Nested.bs"), "package Nested where\n").unwrap();

        let resolution = resolve_local_dependencies(&root, &BTreeSet::from(["Main.bs".to_owned()]));
        let _ = fs::remove_dir_all(&root);

        assert!(resolution.diagnostics.is_empty());
        assert_eq!(
            resolution.paths,
            BTreeSet::from([
                "Local.bs".to_owned(),
                "Main.bs".to_owned(),
                "Nested.bs".to_owned(),
            ])
        );
    }

    #[test]
    fn leaves_missing_roots_to_fixture_validation() {
        let root = temporary_directory();
        let resolution =
            resolve_local_dependencies(&root, &BTreeSet::from(["GeneratedLater.bsv".to_owned()]));
        let _ = fs::remove_dir_all(&root);

        assert!(resolution.diagnostics.is_empty());
        assert_eq!(
            resolution.paths,
            BTreeSet::from(["GeneratedLater.bsv".to_owned()])
        );
    }

    #[test]
    fn rejects_ambiguous_local_packages() {
        let root = temporary_directory();
        fs::create_dir_all(root.join("one")).unwrap();
        fs::create_dir_all(root.join("two")).unwrap();
        fs::write(root.join("Main.bsv"), "import Local::*;\n").unwrap();
        fs::write(root.join("one/Local.bsv"), "").unwrap();
        fs::write(root.join("two/Local.bs"), "").unwrap();

        let resolution =
            resolve_local_dependencies(&root, &BTreeSet::from(["Main.bsv".to_owned()]));
        let _ = fs::remove_dir_all(&root);

        assert_eq!(resolution.diagnostics.len(), 1);
        assert!(resolution.diagnostics[0].contains("ambiguous"));
        assert_eq!(resolution.paths, BTreeSet::from(["Main.bsv".to_owned()]));
    }
}
