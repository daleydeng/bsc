use super::{
    count_diagnostics, is_safe_relative, normalize_golden_output, ArtifactAssertion,
    ArtifactNormalization, TextAssertion,
};
use crate::{normalize_generated_ids, readable_diff};
use regex::Regex;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

pub(super) fn validate_artifact_assertions(
    assertions: &[ArtifactAssertion],
    fixtures: &[&str],
    context: &str,
) -> Result<(), String> {
    for assertion in assertions {
        if !is_safe_relative(assertion.actual_path()) {
            return Err(format!(
                "{context} contains unsafe artifact path {}",
                assertion.actual_path()
            ));
        }
        if let Some(expected) = assertion.expected_path() {
            if !is_safe_relative(expected) || !fixtures.contains(&expected) {
                return Err(format!(
                    "{context} must declare expected artifact {expected} as a fixture"
                ));
            }
            if assertion.actual_path() == expected {
                return Err(format!(
                    "{context} compares artifact {expected} with itself"
                ));
            }
        }
        if let ArtifactAssertion::Matches {
            normalization:
                ArtifactNormalization::DecimalTolerance {
                    fractional_digits, ..
                },
            ..
        } = *assertion
        {
            if !(1..=18).contains(&fractional_digits) {
                return Err(format!(
                    "{context} uses unsupported decimal precision {fractional_digits}; expected 1..=18"
                ));
            }
        }
        if let ArtifactAssertion::Text { assertion, .. } = *assertion {
            let pattern = match assertion {
                TextAssertion::Regex { pattern }
                | TextAssertion::RegexDoesNotMatch { pattern }
                | TextAssertion::RegexCount { pattern, .. } => Some(pattern),
                TextAssertion::Contains { .. }
                | TextAssertion::DoesNotContain { .. }
                | TextAssertion::LineCount { .. }
                | TextAssertion::DiagnosticCount { .. } => None,
            };
            if let Some(pattern) = pattern {
                compile_multiline_regex(pattern)
                    .map_err(|error| format!("{context} contains {error}"))?;
            }
        }
    }
    Ok(())
}

pub(super) fn check_artifact_assertions(
    assertions: &[ArtifactAssertion],
    work_dir: &Path,
    artifact_dir: &Path,
    context: &str,
) -> Result<(), String> {
    for (index, assertion) in assertions.iter().copied().enumerate() {
        match assertion {
            ArtifactAssertion::Exists { path } => {
                let path = work_dir.join(path);
                if !path.is_file() {
                    return Err(format!(
                        "expected artifact {} for {context}",
                        path.display()
                    ));
                }
            }
            ArtifactAssertion::Text { path, assertion } => {
                let path = work_dir.join(path);
                let text = fs::read_to_string(&path).map_err(|error| {
                    format!(
                        "read asserted artifact {} for {context}: {error}",
                        path.display()
                    )
                })?;
                check_text_assertion(&text, assertion).map_err(|error| {
                    format!("artifact assertion failed for {}: {error}", path.display())
                })?;
            }
            ArtifactAssertion::ParsesAsSystemVerilog { path } => {
                let path = work_dir.join(path);
                let include_paths = [path.parent().unwrap_or(Path::new("."))];
                sv_parser::parse_sv(
                    &path,
                    &sv_parser::Defines::new(),
                    &include_paths,
                    true,
                    false,
                )
                .map_err(|error| {
                    format!(
                        "SystemVerilog parser rejected asserted artifact {} for {context}: {error}",
                        path.display()
                    )
                })?;
            }
            ArtifactAssertion::Matches {
                actual,
                expected,
                normalization,
            } => {
                compare_artifacts(
                    &work_dir.join(actual),
                    &work_dir.join(expected),
                    normalization,
                    &artifact_dir.join(format!("artifact-{index}.diff")),
                )
                .map_err(|error| format!("artifact comparison failed for {context}: {error}"))?;
            }
        }
    }
    Ok(())
}

pub(super) fn check_text_assertion(text: &str, assertion: TextAssertion) -> Result<(), String> {
    let normalized = text
        .contains('\r')
        .then(|| text.replace("\r\n", "\n").replace('\r', "\n"));
    let text = normalized.as_deref().unwrap_or(text);

    match assertion {
        TextAssertion::Contains { text: expected } => {
            if !text.contains(expected) {
                return Err(format!("expected to contain {expected:?}"));
            }
        }
        TextAssertion::DoesNotContain { text: unexpected } => {
            if text.contains(unexpected) {
                return Err(format!("expected not to contain {unexpected:?}"));
            }
        }
        TextAssertion::LineCount {
            text: expected,
            count,
        } => {
            let actual = text.lines().filter(|line| line.contains(expected)).count();
            if actual != count {
                return Err(format!(
                    "expected {count} lines containing {expected:?}, found {actual}"
                ));
            }
        }
        TextAssertion::Regex { pattern } => {
            let regex = compile_multiline_regex(pattern)?;
            if !regex.is_match(text) {
                return Err(format!("expected regex {pattern:?} to match"));
            }
        }
        TextAssertion::RegexDoesNotMatch { pattern } => {
            let regex = compile_multiline_regex(pattern)?;
            let actual = regex.find_iter(text).count();
            if actual != 0 {
                return Err(format!(
                    "expected regex {pattern:?} not to match, found {actual} matches"
                ));
            }
        }
        TextAssertion::RegexCount { pattern, count } => {
            let regex = compile_multiline_regex(pattern)?;
            let actual = regex.find_iter(text).count();
            if actual != count {
                return Err(format!(
                    "expected regex {pattern:?} to match {count} times, found {actual}"
                ));
            }
        }
        TextAssertion::DiagnosticCount { kind, tag, count } => {
            let actual = count_diagnostics(text, kind, tag);
            if actual != count {
                return Err(format!(
                    "expected {count} copies of {} {tag}, found {actual}",
                    kind.as_str()
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn compare_golden_output(
    actual: &str,
    expected_path: &Path,
    actual_path: &Path,
    diff_path: &Path,
) -> Result<(), String> {
    compare_golden_output_with(actual, expected_path, actual_path, diff_path, str::to_owned)
}

pub(super) fn compare_golden_output_with(
    actual: &str,
    expected_path: &Path,
    actual_path: &Path,
    diff_path: &Path,
    normalize: impl Fn(&str) -> String,
) -> Result<(), String> {
    let expected = fs::read_to_string(expected_path)
        .map_err(|error| format!("read golden {}: {error}", expected_path.display()))?;
    let actual = normalize(actual);
    let expected = normalize(&expected);
    compare_normalized_text(
        &actual,
        &expected,
        ArtifactNormalization::GoldenOutput,
        actual_path,
        expected_path,
        diff_path,
    )
}

fn compare_artifacts(
    actual_path: &Path,
    expected_path: &Path,
    normalization: ArtifactNormalization,
    diff_path: &Path,
) -> Result<(), String> {
    let actual = fs::read(actual_path)
        .map_err(|error| format!("read actual artifact {}: {error}", actual_path.display()))?;
    let expected = fs::read(expected_path).map_err(|error| {
        format!(
            "read expected artifact {}: {error}",
            expected_path.display()
        )
    })?;

    if normalization == ArtifactNormalization::Exact {
        if actual == expected {
            return Ok(());
        }
        let diff = match (std::str::from_utf8(&expected), std::str::from_utf8(&actual)) {
            (Ok(expected), Ok(actual)) => readable_diff(
                expected,
                actual,
                &expected_path.display().to_string(),
                &actual_path.display().to_string(),
            ),
            _ => format!(
                "binary artifacts differ: expected {} bytes, actual {} bytes\n",
                expected.len(),
                actual.len()
            ),
        };
        return write_mismatch(actual_path, expected_path, diff_path, &diff);
    }

    let actual = String::from_utf8(actual).map_err(|error| {
        format!(
            "actual artifact {} is not UTF-8: {error}",
            actual_path.display()
        )
    })?;
    let expected = String::from_utf8(expected).map_err(|error| {
        format!(
            "expected artifact {} is not UTF-8: {error}",
            expected_path.display()
        )
    })?;
    compare_normalized_text(
        &actual,
        &expected,
        normalization,
        actual_path,
        expected_path,
        diff_path,
    )
}

fn compare_normalized_text(
    actual: &str,
    expected: &str,
    normalization: ArtifactNormalization,
    actual_path: &Path,
    expected_path: &Path,
    diff_path: &Path,
) -> Result<(), String> {
    if let ArtifactNormalization::DecimalTolerance {
        fractional_digits,
        max_units,
    } = normalization
    {
        let actual = normalize_golden_output(actual);
        let expected = normalize_golden_output(expected);
        if decimal_text_within_tolerance(&actual, &expected, fractional_digits, max_units)? {
            return Ok(());
        }
        let diff = readable_diff(
            &expected,
            &actual,
            &expected_path.display().to_string(),
            &actual_path.display().to_string(),
        );
        return write_mismatch(actual_path, expected_path, diff_path, &diff);
    }

    let normalize = |text: &str| match normalization {
        ArtifactNormalization::Exact => text.to_owned(),
        ArtifactNormalization::GoldenOutput => normalize_golden_output(text),
        ArtifactNormalization::Verilog => {
            let without_banner = text
                .lines()
                .filter(|line| !line.contains("Bluespec Compiler"))
                .collect::<Vec<_>>()
                .join("\n");
            normalize_golden_output(&normalize_generated_ids(&without_banner))
        }
        ArtifactNormalization::DecimalTolerance { .. } => unreachable!(),
    };
    let actual = normalize(actual);
    let expected = normalize(expected);
    if actual == expected {
        return Ok(());
    }
    let diff = readable_diff(
        &expected,
        &actual,
        &expected_path.display().to_string(),
        &actual_path.display().to_string(),
    );
    write_mismatch(actual_path, expected_path, diff_path, &diff)
}

fn decimal_text_within_tolerance(
    actual: &str,
    expected: &str,
    fractional_digits: u8,
    max_units: u64,
) -> Result<bool, String> {
    static DECIMAL: OnceLock<Regex> = OnceLock::new();
    let decimal = DECIMAL.get_or_init(|| {
        Regex::new(r"(?P<sign>[+-]?)(?P<whole>[0-9]+)\.(?P<fraction>[0-9]+)")
            .expect("decimal token regex is valid")
    });
    let mut actual_tokens = decimal.captures_iter(actual);
    let mut expected_tokens = decimal.captures_iter(expected);
    let mut actual_end = 0;
    let mut expected_end = 0;

    loop {
        match (actual_tokens.next(), expected_tokens.next()) {
            (None, None) => return Ok(actual[actual_end..] == expected[expected_end..]),
            (Some(actual_token), Some(expected_token)) => {
                let actual_match = actual_token.get(0).expect("decimal capture has a match");
                let expected_match = expected_token.get(0).expect("decimal capture has a match");
                if actual[actual_end..actual_match.start()]
                    != expected[expected_end..expected_match.start()]
                {
                    return Ok(false);
                }
                let Some(actual_value) = scaled_decimal(&actual_token, fractional_digits)? else {
                    return Ok(false);
                };
                let Some(expected_value) = scaled_decimal(&expected_token, fractional_digits)?
                else {
                    return Ok(false);
                };
                if actual_value.abs_diff(expected_value) > u128::from(max_units) {
                    return Ok(false);
                }
                actual_end = actual_match.end();
                expected_end = expected_match.end();
            }
            _ => return Ok(false),
        }
    }
}

fn scaled_decimal(
    token: &regex::Captures<'_>,
    fractional_digits: u8,
) -> Result<Option<i128>, String> {
    let fraction = token
        .name("fraction")
        .expect("decimal capture has a fraction")
        .as_str();
    if fraction.len() != usize::from(fractional_digits) {
        return Ok(None);
    }
    let whole = token
        .name("whole")
        .expect("decimal capture has a whole part")
        .as_str()
        .parse::<i128>()
        .map_err(|error| format!("parse decimal whole part: {error}"))?;
    let fraction = fraction
        .parse::<i128>()
        .map_err(|error| format!("parse decimal fractional part: {error}"))?;
    let scale = 10_i128
        .checked_pow(u32::from(fractional_digits))
        .ok_or_else(|| "decimal scale overflow".to_owned())?;
    let magnitude = whole
        .checked_mul(scale)
        .and_then(|value| value.checked_add(fraction))
        .ok_or_else(|| "decimal value overflow".to_owned())?;
    Ok(Some(
        if token.name("sign").is_some_and(|sign| sign.as_str() == "-") {
            -magnitude
        } else {
            magnitude
        },
    ))
}

fn write_mismatch(
    actual_path: &Path,
    expected_path: &Path,
    diff_path: &Path,
    diff: &str,
) -> Result<(), String> {
    fs::write(diff_path, diff)
        .map_err(|error| format!("write artifact diff {}: {error}", diff_path.display()))?;
    Err(format!(
        "{} differs from {}; see {}",
        actual_path.display(),
        expected_path.display(),
        diff_path.display()
    ))
}

fn compile_multiline_regex(pattern: &str) -> Result<Regex, String> {
    Regex::new(&format!("(?m:{pattern})"))
        .map_err(|error| format!("invalid multiline regex {pattern:?}: {error}"))
}
