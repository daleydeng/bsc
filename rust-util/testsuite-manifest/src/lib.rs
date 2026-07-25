use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::ops::Range;
use std::path::{Path, PathBuf};

use thiserror::Error;
use tree_sitter::{Node, Parser, Tree};
use walkdir::WalkDir;

mod lower;
pub mod model;

pub use model::{ManifestSummary, TestsuiteManifest};

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("could not load the Tree-sitter Tcl grammar: {0}")]
    Grammar(#[from] tree_sitter::LanguageError),
    #[error("could not walk the testsuite: {0}")]
    Walk(#[from] walkdir::Error),
    #[error("could not read {path}: {source}")]
    Read { path: PathBuf, source: io::Error },
    #[error("Tree-sitter cancelled parsing {0}")]
    ParseCancelled(PathBuf),
    #[error("could not serialize the testsuite manifest: {0}")]
    Serialize(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxIssueKind {
    Error,
    Missing,
}

impl SyntaxIssueKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Error => "ERROR",
            Self::Missing => "MISSING",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxIssueScope {
    Structural,
    OpaqueDataArgument,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxIssue {
    pub path: PathBuf,
    pub scope: SyntaxIssueScope,
    pub kind: SyntaxIssueKind,
    pub node_kind: String,
    pub ancestors: Vec<String>,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SyntaxReport {
    pub scripts: usize,
    pub bytes: u64,
    pub opaque_arguments: usize,
    pub normalization_rewrites: usize,
    pub node_kinds: BTreeMap<String, usize>,
    pub issues: Vec<SyntaxIssue>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ParseAdjustments {
    pub opaque_arguments: usize,
    pub normalization_rewrites: usize,
}

pub struct TclParser {
    parser: Parser,
}

impl TclParser {
    pub fn new() -> Result<Self, ManifestError> {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_tcl::LANGUAGE.into())?;
        Ok(Self { parser })
    }

    pub fn parse(&mut self, source: &[u8], path: &Path) -> Result<Tree, ManifestError> {
        let terminated;
        let source = if source.ends_with(b"\n") || source.ends_with(b";") {
            source
        } else {
            terminated = with_trailing_newline(source);
            &terminated
        };
        self.parse_terminated(source, path)
    }

    pub fn parse_contract(
        &mut self,
        source: &[u8],
        path: &Path,
    ) -> Result<(Tree, ParseAdjustments), ManifestError> {
        let (tree, adjustments, _) = self.parse_contract_normalized(source, path)?;
        Ok((tree, adjustments))
    }

    fn parse_contract_normalized(
        &mut self,
        source: &[u8],
        path: &Path,
    ) -> Result<(Tree, ParseAdjustments, Vec<u8>), ManifestError> {
        let mut normalized = with_trailing_newline(source);
        let line_continuation_rewrites = normalize_line_continuations(&mut normalized);
        let variable_rewrites = normalize_variable_forms(&mut normalized);
        let mut opaque_ranges = collect_allowlisted_opaque_ranges(source)
            .into_iter()
            .map(|range| (range.start, range.end))
            .collect::<BTreeSet<_>>();
        for &(start, end) in &opaque_ranges {
            mask_range(&mut normalized, start + 1..end - 1);
        }
        let mut normalization_ranges = BTreeSet::new();
        loop {
            let tree = self.parse_terminated(&normalized, path)?;

            let mut discovered_opaque = Vec::new();
            collect_opaque_ranges(tree.root_node(), source, &mut discovered_opaque);
            let new_opaque = discovered_opaque
                .into_iter()
                .filter(|range| opaque_ranges.insert((range.start, range.end)))
                .collect::<Vec<_>>();

            let mut discovered_normalization = Vec::new();
            collect_normalization_ranges(tree.root_node(), source, &mut discovered_normalization);
            let new_normalization = discovered_normalization
                .into_iter()
                .filter(|range| normalization_ranges.insert((range.start, range.end)))
                .collect::<Vec<_>>();

            if new_opaque.is_empty() && new_normalization.is_empty() {
                return Ok((
                    tree,
                    ParseAdjustments {
                        opaque_arguments: opaque_ranges.len(),
                        normalization_rewrites: line_continuation_rewrites
                            + variable_rewrites
                            + normalization_ranges.len(),
                    },
                    normalized,
                ));
            }
            for range in new_opaque {
                mask_range(&mut normalized, range.start + 1..range.end - 1);
            }
            for range in new_normalization {
                mask_range(&mut normalized, range);
            }
        }
    }

    pub fn cst(&mut self, source: &[u8], path: &Path) -> Result<String, ManifestError> {
        Ok(self.parse(source, path)?.root_node().to_sexp())
    }

    fn parse_terminated(&mut self, source: &[u8], path: &Path) -> Result<Tree, ManifestError> {
        self.parser
            .parse(source, None)
            .ok_or_else(|| ManifestError::ParseCancelled(path.to_owned()))
    }
}

pub fn render_manifest(manifest: &TestsuiteManifest) -> Result<String, ManifestError> {
    let mut rendered = serde_json::to_string_pretty(manifest)?;
    rendered.push('\n');
    Ok(rendered)
}

pub fn build_manifest(project_root: &Path) -> Result<TestsuiteManifest, ManifestError> {
    let mut parser = TclParser::new()?;
    let mut scripts = Vec::new();
    for path in contract_script_paths(project_root)? {
        let source = fs::read(&path).map_err(|source| ManifestError::Read {
            path: path.clone(),
            source,
        })?;
        let (tree, _) = parser.parse_contract(&source, &path)?;
        let origin = path
            .strip_prefix(project_root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        scripts.push(lower::lower_script(origin, &source, &tree));
    }
    Ok(TestsuiteManifest {
        schema_version: model::MANIFEST_SCHEMA_VERSION,
        scripts,
    })
}

pub fn scan_testsuite(project_root: &Path) -> Result<SyntaxReport, ManifestError> {
    let scripts = contract_script_paths(project_root)?;
    let mut parser = TclParser::new()?;
    let mut report = SyntaxReport::default();
    for path in scripts {
        let source = fs::read(&path).map_err(|source| ManifestError::Read {
            path: path.clone(),
            source,
        })?;
        let (tree, adjustments) = parser.parse_contract(&source, &path)?;
        let relative_path = path.strip_prefix(project_root).unwrap_or(&path).to_owned();

        report.scripts += 1;
        report.bytes += source.len() as u64;
        report.opaque_arguments += adjustments.opaque_arguments;
        report.normalization_rewrites += adjustments.normalization_rewrites;
        inspect_node(
            tree.root_node(),
            &relative_path,
            &source,
            false,
            &mut report.node_kinds,
            &mut report.issues,
        );
    }
    Ok(report)
}

fn contract_script_paths(project_root: &Path) -> Result<Vec<PathBuf>, ManifestError> {
    let testsuite_root = project_root.join("testsuite");
    let mut scripts = WalkDir::new(&testsuite_root)
        .follow_links(false)
        .into_iter()
        .filter_map(|entry| match entry {
            Ok(entry)
                if entry.file_type().is_file()
                    && entry.path().extension().is_some_and(|ext| ext == "exp")
                    && is_contract_script(project_root, entry.path()) =>
            {
                Some(Ok(entry.into_path()))
            }
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .collect::<Result<Vec<_>, _>>()?;
    scripts.sort();
    Ok(scripts)
}

fn with_trailing_newline(source: &[u8]) -> Vec<u8> {
    let mut terminated = Vec::with_capacity(source.len() + 1);
    terminated.extend_from_slice(source);
    if !source.ends_with(b"\n") && !source.ends_with(b";") {
        terminated.push(b'\n');
    }
    terminated
}

const OPAQUE_BRACED_HELPERS: &[&[u8]] = &[
    b"find_regexp",
    b"find_n_regexp",
    b"find_n_strings",
    b"string_does_not_occur",
    b"string_occurs",
];

fn collect_allowlisted_opaque_ranges(source: &[u8]) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut line_start = 0;
    while line_start < source.len() {
        let mut command_start = line_start;
        while source
            .get(command_start)
            .is_some_and(|byte| matches!(*byte, b' ' | b'\t' | b'\r'))
        {
            command_start += 1;
        }
        if let Some(helper) = OPAQUE_BRACED_HELPERS.iter().find(|helper| {
            source[command_start..].starts_with(helper)
                && source
                    .get(command_start + helper.len())
                    .is_some_and(u8::is_ascii_whitespace)
        }) {
            if let Some(range) =
                first_braced_word_in_logical_command(source, command_start + helper.len())
            {
                ranges.push(range);
            }
        }
        line_start = source[line_start..]
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(source.len(), |newline| line_start + newline + 1);
    }
    ranges
}

fn first_braced_word_in_logical_command(source: &[u8], mut index: usize) -> Option<Range<usize>> {
    while index < source.len() {
        match source[index] {
            b'\\' if source.get(index + 1) == Some(&b'\n') => index += 2,
            b'\\'
                if source.get(index + 1) == Some(&b'\r')
                    && source.get(index + 2) == Some(&b'\n') =>
            {
                index += 3;
            }
            b'\\' => index = (index + 2).min(source.len()),
            b'{' => return balanced_braced_range(source, index),
            b'\n' | b';' => return None,
            _ => index += 1,
        }
    }
    None
}

fn collect_opaque_ranges(node: Node<'_>, source: &[u8], ranges: &mut Vec<Range<usize>>) {
    if is_opaque_data_argument(node) || is_recovered_opaque_argument(node, source) {
        if let Some(range) = balanced_braced_range(source, node.start_byte()) {
            ranges.push(range);
            return;
        }
    }
    if let Some(range) = recovered_generic_command_brace(node, source) {
        ranges.push(range);
        return;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_opaque_ranges(child, source, ranges);
    }
}

fn is_recovered_opaque_argument(node: Node<'_>, source: &[u8]) -> bool {
    if !node.is_error() || source.get(node.start_byte()) != Some(&b'{') {
        return false;
    }
    let Some(previous) = node.prev_named_sibling() else {
        return false;
    };
    previous.kind() == "command"
        && previous.end_position().row == node.start_position().row
        && source[previous.end_byte()..node.start_byte()]
            .iter()
            .all(|byte| byte.is_ascii_whitespace() || *byte == b'\\')
}

fn recovered_generic_command_brace(node: Node<'_>, source: &[u8]) -> Option<Range<usize>> {
    if !node.is_error() {
        return None;
    }
    let command_name = node.named_child(0)?;
    if command_name.kind() != "simple_word" {
        return None;
    }
    let name = source.get(command_name.byte_range())?;
    if matches!(
        name,
        b"if" | b"while" | b"for" | b"foreach" | b"proc" | b"catch" | b"try" | b"switch"
    ) {
        return None;
    }

    let mut index = command_name.end_byte();
    let mut bracket_depth = 0usize;
    while index < node.end_byte().min(source.len()) {
        match source[index] {
            b'\\' if source.get(index + 1) == Some(&b'\n') => index += 2,
            b'\\'
                if source.get(index + 1) == Some(&b'\r')
                    && source.get(index + 2) == Some(&b'\n') =>
            {
                index += 3;
            }
            b'\\' => index = (index + 2).min(source.len()),
            b'[' => {
                bracket_depth += 1;
                index += 1;
            }
            b']' => {
                bracket_depth = bracket_depth.saturating_sub(1);
                index += 1;
            }
            b'{' if bracket_depth == 0 => return balanced_braced_range(source, index),
            b'\n' | b';' if bracket_depth == 0 => return None,
            _ => index += 1,
        }
    }
    None
}

fn collect_normalization_ranges(node: Node<'_>, source: &[u8], ranges: &mut Vec<Range<usize>>) {
    if node.is_error()
        && source.get(node.byte_range()) == Some(b"then")
        && std::iter::successors(node.parent(), |parent| parent.parent())
            .any(|ancestor| ancestor.kind() == "if")
    {
        ranges.push(node.byte_range());
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_normalization_ranges(child, source, ranges);
    }
}

fn normalize_line_continuations(source: &mut [u8]) -> usize {
    let mut rewrites = 0;
    let mut index = 0;
    while index + 1 < source.len() {
        if source[index] == b'\\' && source[index + 1] == b'\n' {
            source[index] = b' ';
            source[index + 1] = b' ';
            rewrites += 1;
            index += 2;
        } else if index + 2 < source.len()
            && source[index] == b'\\'
            && source[index + 1] == b'\r'
            && source[index + 2] == b'\n'
        {
            source[index] = b' ';
            source[index + 1] = b' ';
            source[index + 2] = b' ';
            rewrites += 1;
            index += 3;
        } else {
            index += 1;
        }
    }
    rewrites
}

fn normalize_variable_forms(source: &mut [u8]) -> usize {
    let mut rewrites = 0;
    let mut index = 0;
    while index < source.len() {
        if source[index] == b'$' && source.get(index + 1) != Some(&b'{') {
            let name_start = index + 1;
            let mut cursor = name_start;
            while source
                .get(cursor)
                .is_some_and(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'_' | b':'))
            {
                cursor += 1;
            }
            if source.get(cursor) == Some(&b'(') {
                if let Some(end) = balanced_parenthesized_end(source, cursor) {
                    rewrite_as_identifier(source, name_start..end);
                    rewrites += 1;
                    index = end;
                    continue;
                }
            } else if source.get(name_start..name_start + 2) == Some(b"::") {
                source[name_start] = b'_';
                source[name_start + 1] = b'_';
                rewrites += 1;
            }
        }

        let starts_env_array = source[index..].starts_with(b"env(")
            && (index == 0
                || !matches!(
                    source[index - 1],
                    b'$' | b'_' | b':' | b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9'
                ));
        if starts_env_array {
            if let Some(end) = balanced_parenthesized_end(source, index + 3) {
                rewrite_as_identifier(source, index..end);
                rewrites += 1;
                index = end;
                continue;
            }
        }
        index += 1;
    }
    rewrites
}

fn balanced_parenthesized_end(source: &[u8], start: usize) -> Option<usize> {
    if source.get(start) != Some(&b'(') {
        return None;
    }
    let mut depth = 1usize;
    let mut index = start + 1;
    while index < source.len() {
        match source[index] {
            b'\\' => index = (index + 2).min(source.len()),
            b'(' => {
                depth += 1;
                index += 1;
            }
            b')' => {
                depth -= 1;
                index += 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => index += 1,
        }
    }
    None
}

fn rewrite_as_identifier(source: &mut [u8], range: Range<usize>) {
    for byte in &mut source[range] {
        if !matches!(*byte, b'\n' | b'\r') {
            *byte = b'v';
        }
    }
}

fn mask_range(source: &mut [u8], range: Range<usize>) {
    for byte in &mut source[range] {
        if !matches!(*byte, b'\n' | b'\r') {
            *byte = b' ';
        }
    }
}

fn balanced_braced_range(source: &[u8], start: usize) -> Option<Range<usize>> {
    if source.get(start) != Some(&b'{') {
        return None;
    }
    let mut depth = 1usize;
    let mut index = start + 1;
    while index < source.len() {
        match source[index] {
            b'\\' => index = (index + 2).min(source.len()),
            b'{' => {
                depth += 1;
                index += 1;
            }
            b'}' => {
                depth -= 1;
                index += 1;
                if depth == 0 {
                    return Some(start..index);
                }
            }
            _ => index += 1,
        }
    }
    None
}

fn is_contract_script(project_root: &Path, path: &Path) -> bool {
    let relative = path.strip_prefix(project_root).unwrap_or(path);
    ![
        Path::new("testsuite/config/unix.exp"),
        Path::new("testsuite/lib/bsc.exp"),
        Path::new("testsuite/site.exp"),
    ]
    .contains(&relative)
}

fn inspect_node(
    node: Node<'_>,
    path: &Path,
    source: &[u8],
    inside_opaque_argument: bool,
    node_kinds: &mut BTreeMap<String, usize>,
    issues: &mut Vec<SyntaxIssue>,
) {
    *node_kinds.entry(node.kind().to_owned()).or_default() += 1;
    let inside_opaque_argument = inside_opaque_argument || is_opaque_data_argument(node);

    let issue_kind = if node.is_error() {
        Some(SyntaxIssueKind::Error)
    } else if node.is_missing() {
        Some(SyntaxIssueKind::Missing)
    } else {
        None
    };
    if let Some(kind) = issue_kind {
        let (start_line, start_column) = source_position(source, node.start_byte());
        let (end_line, end_column) = source_position(source, node.end_byte());
        issues.push(SyntaxIssue {
            path: path.to_owned(),
            scope: if inside_opaque_argument {
                SyntaxIssueScope::OpaqueDataArgument
            } else {
                SyntaxIssueScope::Structural
            },
            kind,
            node_kind: node.kind().to_owned(),
            ancestors: std::iter::successors(node.parent(), |parent| parent.parent())
                .take(6)
                .map(|ancestor| ancestor.kind().to_owned())
                .collect(),
            start_line,
            start_column,
            end_line,
            end_column,
        });
        return;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        inspect_node(
            child,
            path,
            source,
            inside_opaque_argument,
            node_kinds,
            issues,
        );
    }
}

fn source_position(source: &[u8], offset: usize) -> (usize, usize) {
    let prefix = &source[..offset.min(source.len())];
    let line = prefix.iter().filter(|byte| **byte == b'\n').count() + 1;
    let column = prefix
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(prefix.len() + 1, |newline| prefix.len() - newline);
    (line, column)
}

fn is_opaque_data_argument(node: Node<'_>) -> bool {
    if !matches!(node.kind(), "braced_word" | "braced_word_simple") {
        return false;
    }
    let Some(parent) = node.parent() else {
        return false;
    };
    if matches!(parent.kind(), "set" | "regexp") {
        return true;
    }
    node.kind() == "braced_word"
        && parent.kind() == "word_list"
        && parent
            .parent()
            .is_some_and(|command| command.kind() == "command")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_parses(source: &str) {
        let mut parser = TclParser::new().expect("load Tcl grammar");
        let tree = parser
            .parse(source.as_bytes(), Path::new("fixture.exp"))
            .expect("parse Tcl fixture");
        assert!(
            !tree.root_node().has_error(),
            "{}",
            tree.root_node().to_sexp()
        );
    }

    #[test]
    fn parses_testsuite_style_tcl_structures() {
        assert_parses(
            r#"
set modules { MkOne
              MkTwo }
if {$ctest == 1} {
    compile_pass $modules
}
set output [make_bsc_output_name "${module}.bsv"]
regexp {Warning: \$display} $output
"#,
        );
    }

    #[test]
    fn exposes_a_structured_cst() {
        let mut parser = TclParser::new().expect("load Tcl grammar");
        let cst = parser
            .cst(b"set modules {MkOne MkTwo}\n", Path::new("fixture.exp"))
            .expect("render Tcl CST");
        assert!(cst.contains("(set "), "{cst}");
        assert!(cst.contains("simple_word"), "{cst}");
    }

    fn assert_contract_parses(source: &str) {
        let mut parser = TclParser::new().expect("load Tcl grammar");
        let (tree, _) = parser
            .parse_contract(source.as_bytes(), Path::new("fixture.exp"))
            .expect("parse contract fixture");
        assert!(
            !tree.root_node().has_error(),
            "{}",
            tree.root_node().to_sexp()
        );
    }

    fn issues_for(source: &str) -> Vec<SyntaxIssue> {
        let mut parser = TclParser::new().expect("load Tcl grammar");
        let tree = parser
            .parse(source.as_bytes(), Path::new("fixture.exp"))
            .expect("parse Tcl fixture");
        let mut node_kinds = BTreeMap::new();
        let mut issues = Vec::new();
        inspect_node(
            tree.root_node(),
            Path::new("fixture.exp"),
            source.as_bytes(),
            false,
            &mut node_kinds,
            &mut issues,
        );
        issues
    }

    #[test]
    fn parses_multiline_helpers_with_opaque_data() {
        assert_contract_parses(
            "find_regexp ECtx.bsc-out \\\n    {ModBindInAVBlock\\.bsv\", line 3, column 17:}\n",
        );
        assert_contract_parses(
            "find_n_strings [make_bsc_vcomp_output_name Fixup.bsv] \\\n    {method (m, [])m enable ((EN_m, [])) clocked_by (no_clock);} 1\n",
        );
    }

    #[test]
    fn parses_multiline_opaque_data_inside_a_capability_gate() {
        assert_contract_parses(
            r#"if { $vtest == 1 } {
compile_verilog_pass StringVecDisplay.bsv
find_regexp sysStringVecDisplay.v \
    {if \(idx \=\= 2\'d0\)
      \$display\(\"Hello\"\)\;
    else
      \$display\(\"World\!\"\)\;}
compile_verilog_pass StringVecParam.bsv
find_regexp sysStringVecParam.v \
    {mkStringVecParam_Sub \#\(\.str\(\(idx \=\= 2\'d0\) \?
      \"Hello\" \:
      \"Bye\"\)\)\;}
}
"#,
        );
    }

    #[test]
    fn treats_custom_helper_braced_data_as_opaque() {
        let source =
            r#"find_regexp output.v {if \(RST_N != `BSV_RESET_VALUE\) \$display\(\"Hi\"\)\;}"#;
        let raw_issues = issues_for(source);
        assert!(
            raw_issues
                .iter()
                .any(|issue| issue.scope == SyntaxIssueScope::OpaqueDataArgument),
            "fixture must exercise the grammar gap: {raw_issues:#?}"
        );

        let mut parser = TclParser::new().expect("load Tcl grammar");
        let (tree, adjustments) = parser
            .parse_contract(source.as_bytes(), Path::new("fixture.exp"))
            .expect("parse contract fixture");
        assert_eq!(adjustments.opaque_arguments, 1);
        assert!(
            !tree.root_node().has_error(),
            "{}",
            tree.root_node().to_sexp()
        );
    }

    #[test]
    fn balances_nested_and_escaped_opaque_braces() {
        let source = br#"{outer {nested} \{escaped\}} trailing"#;
        assert_eq!(balanced_braced_range(source, 0), Some(0..28));
    }

    #[test]
    fn keeps_control_structure_errors_strict() {
        let issues = issues_for("if {$vtest == 1} {\n    compile_pass Test.bsv\n");
        assert!(
            issues
                .iter()
                .any(|issue| issue.scope == SyntaxIssueScope::Structural),
            "{issues:#?}"
        );
    }

    #[test]
    fn parses_dynamic_array_contract_script() {
        let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("manifest crate lives under util/");
        let path = project_root.join("testsuite/bsc.arrays/dynamic/arrays_dynamic.exp");
        let source = fs::read(&path).expect("read dynamic array contract script");
        let mut parser = TclParser::new().expect("load Tcl grammar");
        let (tree, _, normalized) = parser
            .parse_contract_normalized(&source, &path)
            .expect("parse dynamic array contract script");
        if tree.root_node().has_error() {
            let excerpt = String::from_utf8_lossy(&normalized)
                .lines()
                .enumerate()
                .skip(185)
                .take(45)
                .map(|(index, line)| format!("{:4}: {line}", index + 1))
                .collect::<Vec<_>>()
                .join("\n");
            panic!(
                "{}\nnormalized excerpt:\n{excerpt}",
                tree.root_node().to_sexp()
            );
        }
    }

    #[test]
    fn parses_representative_upstream_contract_scripts() {
        let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("manifest crate lives under util/");
        let scripts = [
            "testsuite/bsc.bugs/bluespec_inc/b1018/b1018.exp",
            "testsuite/bsc.bsv_examples/AmbaAdapters/amba_adapters.exp",
            "testsuite/bsc.real/evaluator/evaluator.exp",
            "testsuite/bsc.lib/PAClib/dft64/bsv/paclib_dft.exp",
        ];
        let mut parser = TclParser::new().expect("load Tcl grammar");
        for script in scripts {
            let path = project_root.join(script);
            let source = fs::read(&path).unwrap_or_else(|error| {
                panic!("read representative script {}: {error}", path.display())
            });
            let tree = parser
                .parse(&source, &path)
                .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()));
            assert!(
                !tree.root_node().has_error(),
                "Tree-sitter error in {script}: {}",
                tree.root_node().to_sexp()
            );
        }
    }
}
