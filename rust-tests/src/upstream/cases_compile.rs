use super::{CaseModule, CompileCase};
use std::sync::OnceLock;

macro_rules! compile_pass_case {
    ($name:expr, $fixture_dir:expr, $source:expr) => {
        $crate::upstream::CompileCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: &[$source],
            expectation: $crate::upstream::CompileExpectation::Pass,
            golden: None,
            options: &[],
            nodeps: false,
            mode: $crate::upstream::CompileMode::Frontend,
            requirement: $crate::upstream::Requirement::Always,
        }
    };
}

macro_rules! compile_fail_case {
    ($name:expr, $fixture_dir:expr, $source:expr) => {
        $crate::upstream::CompileCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: &[$source],
            expectation: $crate::upstream::CompileExpectation::Fail,
            golden: None,
            options: &[],
            nodeps: false,
            mode: $crate::upstream::CompileMode::Frontend,
            requirement: $crate::upstream::Requirement::Always,
        }
    };
}

macro_rules! compile_fail_error_case {
    ($name:expr, $fixture_dir:expr, $source:expr, $tag:expr) => {
        $crate::upstream::CompileCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: &[$source],
            expectation: $crate::upstream::CompileExpectation::FailWithDiagnostic {
                kind: $crate::upstream::DiagnosticKind::Error,
                tag: $tag,
                count: 1,
            },
            golden: None,
            options: &[],
            nodeps: false,
            mode: $crate::upstream::CompileMode::Frontend,
            requirement: $crate::upstream::Requirement::Always,
        }
    };
}

macro_rules! compile_fail_golden_case {
    ($name:expr, $fixture_dir:expr, $source:expr, $golden:expr) => {
        $crate::upstream::CompileCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: &[$source, $golden],
            expectation: $crate::upstream::CompileExpectation::Fail,
            golden: Some($crate::upstream::GoldenExpectation { expected: $golden }),
            options: &[],
            nodeps: false,
            mode: $crate::upstream::CompileMode::Frontend,
            requirement: $crate::upstream::Requirement::Always,
        }
    };
}

macro_rules! compile_fail_error_golden_case {
    ($name:literal, $fixture_dir:literal, $source:literal, $tag:literal, $golden:literal) => {
        $crate::upstream::CompileCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: &[$source, $golden],
            expectation: $crate::upstream::CompileExpectation::FailWithDiagnostic {
                kind: $crate::upstream::DiagnosticKind::Error,
                tag: $tag,
                count: 1,
            },
            golden: Some($crate::upstream::GoldenExpectation { expected: $golden }),
            options: &[],
            nodeps: false,
            mode: $crate::upstream::CompileMode::Frontend,
            requirement: $crate::upstream::Requirement::Always,
        }
    };
}

macro_rules! compile_verilog_pass_case {
    ($name:literal, $fixture_dir:literal, $source:literal) => {
        $crate::upstream::CompileCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: &[$source],
            expectation: $crate::upstream::CompileExpectation::Pass,
            golden: None,
            options: &[],
            nodeps: false,
            mode: $crate::upstream::CompileMode::Verilog { module: None },
            requirement: $crate::upstream::Requirement::VerilogEnabled,
        }
    };
}

macro_rules! compile_verilog_pass_warning_case {
    ($name:expr, $fixture_dir:expr, $source:expr, $tag:expr) => {
        $crate::upstream::CompileCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: &[$source],
            expectation: $crate::upstream::CompileExpectation::PassWithDiagnostic {
                kind: $crate::upstream::DiagnosticKind::Warning,
                tag: $tag,
                count: 1,
            },
            golden: None,
            options: &[],
            nodeps: false,
            mode: $crate::upstream::CompileMode::Verilog { module: None },
            requirement: $crate::upstream::Requirement::VerilogEnabled,
        }
    };
}

macro_rules! compile_verilog_fail_error_case {
    ($name:expr, $fixture_dir:expr, $source:expr, $tag:expr) => {
        $crate::upstream::CompileCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: &[$source],
            expectation: $crate::upstream::CompileExpectation::FailWithDiagnostic {
                kind: $crate::upstream::DiagnosticKind::Error,
                tag: $tag,
                count: 1,
            },
            golden: None,
            options: &[],
            nodeps: false,
            mode: $crate::upstream::CompileMode::Verilog { module: None },
            requirement: $crate::upstream::Requirement::VerilogEnabled,
        }
    };
}

macro_rules! compile_verilog_fail_golden_case {
    ($name:expr, $fixture_dir:expr, $source:expr, $golden:expr) => {
        $crate::upstream::CompileCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: &[$source, $golden],
            expectation: $crate::upstream::CompileExpectation::Fail,
            golden: Some($crate::upstream::GoldenExpectation { expected: $golden }),
            options: &[],
            nodeps: false,
            mode: $crate::upstream::CompileMode::Verilog { module: None },
            requirement: $crate::upstream::Requirement::VerilogEnabled,
        }
    };
}

macro_rules! case_modules {
    ($($module:ident),+ $(,)?) => {
        $(mod $module;)+

        pub(super) const MODULES: &[CaseModule<CompileCase>] = &[
            $(CaseModule {
                name: stringify!($module),
                cases: $module::CASES,
            },)+
        ];
    };
}

case_modules!(
    attr_errors,
    b235,
    b810,
    bluespec_inc_fail,
    bluespec_inc_golden,
    bluespec_inc_golden_mixed,
    bluespec_inc_multi,
    bluespec_inc_pass,
    bluespec_inc_single,
    bound_vars,
    bounds_select,
    bounds_update,
    case_syntax,
    conflict_free,
    cross_suite_basic,
    cross_suite_direct,
    cross_suite_errors,
    cross_suite_golden,
    cross_suite_mixed,
    cross_suite_multi,
    dynamic,
    enot_field,
    infer_kinds,
    read_desugaring,
    small_regressions,
    underscore,
);

pub(super) fn cases() -> &'static [CompileCase] {
    static CASES: OnceLock<Vec<CompileCase>> = OnceLock::new();
    CASES
        .get_or_init(|| {
            MODULES
                .iter()
                .flat_map(|module| module.cases.iter().copied())
                .collect()
        })
        .as_slice()
}
