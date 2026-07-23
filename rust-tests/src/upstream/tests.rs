use super::*;
use std::collections::BTreeSet;

#[test]
fn compile_data_model_is_valid_and_names_are_unique() {
    let cases = compile_cases();
    assert!(!cases.is_empty());

    let names: BTreeSet<_> = cases.iter().map(|case| case.name).collect();
    assert_eq!(names.len(), cases.len(), "case names must be unique");

    for case in cases {
        validate_case(case).unwrap();
        match case.expectation {
            CompileExpectation::PassWithDiagnostic { count, .. }
            | CompileExpectation::FailWithDiagnostic { count, .. } => {
                assert!(
                    count > 0,
                    "diagnostic count must be positive for {}",
                    case.name
                );
            }
            CompileExpectation::Pass | CompileExpectation::Fail => {}
        }
    }

    let canonical_names = [
        "bsc.bugs/bluespec_inc/b600::Bug600.bsv",
        "bsc.bugs/bluespec_inc/b267::Bug267.bs",
        "bsc.bugs/bluespec_inc/b1040::Bug1040.bsv",
        "bsc.bugs/bluespec_inc/b417::Bug417.bsv",
        "bsc.bugs/bluespec_inc/b492::Bug492_1.bs",
        "bsc.bugs/bluespec_inc/b1586::Bug1586.bsv",
        "bsc.bugs/bluespec_inc/b269::Bug269.bsv",
        "bsc.bugs/bluespec_inc/b1493::Bug1493.bsv",
        "bsc.bugs/bluespec_inc/b1493::Bug1493_Bad.bsv",
    ];
    assert!(canonical_names.into_iter().all(|name| names.contains(name)));
}

#[test]
fn compile_modes_build_distinct_unix_exp_argv() {
    let cases = compile_cases();
    let mut frontend = *cases
        .iter()
        .find(|case| matches!(case.mode, CompileMode::Frontend))
        .unwrap();
    frontend.options = &["-keep-fires"];
    assert_eq!(
        compile_arguments(&frontend),
        [
            "-keep-fires",
            "-no-show-timestamps",
            "-no-show-version",
            "-u",
            frontend.source,
        ]
    );
    frontend.nodeps = true;
    assert_eq!(
        compile_arguments(&frontend),
        [
            "-keep-fires",
            "-no-show-timestamps",
            "-no-show-version",
            frontend.source,
        ]
    );

    let mut verilog = *cases
        .iter()
        .find(|case| matches!(case.mode, CompileMode::Verilog { .. }))
        .unwrap();
    assert_eq!(
        compile_arguments(&verilog),
        [
            "-no-show-timestamps",
            "-no-show-version",
            "-u",
            "-verilog",
            verilog.source,
        ]
    );
    verilog.mode = CompileMode::Verilog {
        module: Some("mkTop"),
    };
    assert_eq!(
        compile_arguments(&verilog),
        [
            "-no-show-timestamps",
            "-no-show-version",
            "-u",
            "-verilog",
            "-g",
            "mkTop",
            verilog.source,
        ]
    );

    verilog.mode = CompileMode::VerilogSchedule {
        module: Some("mkTop"),
    };
    assert_eq!(
        compile_arguments(&verilog),
        [
            "-no-show-timestamps",
            "-no-show-version",
            "-u",
            "-resource-simple",
            "-show-schedule",
            "-dschedule",
            "-dresources",
            "-dvschedinfo",
            "-verilog",
            "-g",
            "mkTop",
            verilog.source,
        ]
    );
}

#[test]
fn backend_policy_defaults_enabled_and_can_disable_verilog() {
    let cases = compile_cases();
    let default_policy = RunnerPolicy::default();
    assert!(default_policy.verilog_enabled);
    assert!(cases
        .iter()
        .all(|case| default_policy.skip_reason(case.requirement).is_none()));

    let disabled = RunnerPolicy::new(true, false);
    assert!(!disabled.verilog_enabled);
    let skipped: Vec<_> = cases
        .iter()
        .filter(|case| disabled.skip_reason(case.requirement).is_some())
        .collect();
    let verilog_cases = cases
        .iter()
        .filter(|case| {
            matches!(
                case.mode,
                CompileMode::Verilog { .. } | CompileMode::VerilogSchedule { .. }
            )
        })
        .count();
    assert_eq!(skipped.len(), verilog_cases);
    assert!(skipped.iter().all(|case| {
        matches!(
            case.mode,
            CompileMode::Verilog { .. } | CompileMode::VerilogSchedule { .. }
        )
    }));
}

#[test]
fn simulation_data_model_is_valid_and_names_are_unique() {
    let scenarios = simulation_scenarios();
    assert!(!scenarios.is_empty());

    let scenario_names: BTreeSet<_> = scenarios.iter().map(|scenario| scenario.name).collect();
    assert_eq!(
        scenario_names.len(),
        scenarios.len(),
        "scenario names must be unique"
    );
    for scenario in scenarios {
        validate_simulation_scenario(scenario).unwrap();
    }

    let contract_count = scenarios
        .iter()
        .map(|scenario| scenario.contracts.len())
        .sum::<usize>();
    let contract_names = scenarios
        .iter()
        .flat_map(|scenario| scenario.contracts)
        .map(|contract| contract.name)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        contract_names.len(),
        contract_count,
        "simulation contract names must be unique"
    );

    let all_names = compile_cases()
        .iter()
        .map(|case| case.name)
        .chain(contract_names.iter().copied())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        all_names.len(),
        compile_cases().len() + contract_count,
        "all upstream contract names must be unique"
    );
}

#[test]
fn execution_plan_preserves_declared_generation_scenarios() {
    let options = parse_cli(["bsc.bsv_examples/AES::Aes_TB::"]).unwrap();
    let plan = select_plan(&options);
    assert!(plan.compile_cases.is_empty());
    assert_eq!(plan.contract_count(), 2);
    assert_eq!(plan.simulations.len(), 1);
    let aes = &plan.simulations[0];
    assert_eq!(aes.contracts.len(), 2);
    assert_eq!(
        aes.scenario.generation,
        GenerationStrategy::SharedElaboration
    );
    assert_eq!(aes.scenario.resource, ResourceClass::Heavy);
    assert_eq!(
        aes.scenario.timeouts,
        SimulationTimeouts::uniform(crate::BSC_HEAVY_TIMEOUT)
    );

    let options = parse_cli(["bsc.verilog/positivereset/SyncReset::RstTest::"]).unwrap();
    let positive_reset = select_plan(&options);
    assert_eq!(positive_reset.contract_count(), 2);
    assert_eq!(positive_reset.simulations.len(), 2);
}

#[test]
fn backend_policy_skips_disabled_simulation_backends() {
    let requirements = compile_cases()
        .iter()
        .map(|case| case.requirement)
        .chain(
            simulation_scenarios()
                .iter()
                .flat_map(|scenario| scenario.contracts)
                .map(|contract| contract.requirement),
        )
        .collect::<Vec<_>>();
    let no_bluesim = RunnerPolicy::new(false, true).with_iverilog_major(Some(13));
    let skipped_without_bluesim = requirements
        .iter()
        .filter(|requirement| no_bluesim.skip_reason(**requirement).is_some())
        .count();
    let bluesim_cases = requirements
        .iter()
        .filter(|requirement| **requirement == Requirement::BluesimEnabled)
        .count();
    assert_eq!(skipped_without_bluesim, bluesim_cases);

    let no_verilog = RunnerPolicy::new(true, false);
    let skipped_without_verilog = requirements
        .iter()
        .filter(|requirement| no_verilog.skip_reason(**requirement).is_some())
        .count();
    let verilog_cases = requirements
        .iter()
        .filter(|requirement| {
            matches!(
                **requirement,
                Requirement::VerilogEnabled | Requirement::IcarusAtLeast(_)
            )
        })
        .count();
    assert_eq!(skipped_without_verilog, verilog_cases);
}

#[test]
fn iverilog_version_requirements_match_upstream_exclusions() {
    assert_eq!(
        parse_iverilog_major("Icarus Verilog version 11.0 (stable) ()\n"),
        Some(11)
    );

    let version_11 = RunnerPolicy::default().with_iverilog_major(Some(11));
    assert!(version_11
        .skip_reason(Requirement::IcarusAtLeast(12))
        .is_some());
    assert!(version_11
        .skip_reason(Requirement::IcarusAtLeast(13))
        .is_some());

    let version_12 = RunnerPolicy::default().with_iverilog_major(Some(12));
    assert!(version_12
        .skip_reason(Requirement::IcarusAtLeast(12))
        .is_none());
    assert!(version_12
        .skip_reason(Requirement::IcarusAtLeast(13))
        .is_some());

    let version_13 = RunnerPolicy::default().with_iverilog_major(Some(13));
    assert!(version_13
        .skip_reason(Requirement::IcarusAtLeast(13))
        .is_none());
}

#[test]
fn iverilog_output_filter_removes_nondeterministic_noise() {
    let output = concat!(
        "$readmem ignored\n",
        "WARNING: file: $readmem changed\n",
        "keep this\n",
        "foo $finish called\n",
        "VCD info: dumpfile\n"
    );
    assert_eq!(clean_iverilog_output(output), "keep this\n");
}

#[test]
fn outcome_summary_counts_non_failing_outcomes_separately() {
    let outcomes = [
        CaseOutcome {
            name: "pass",
            result: CaseResult::Passed,
        },
        CaseOutcome {
            name: "xfail",
            result: CaseResult::XFailed("known backend defect".to_owned()),
        },
        CaseOutcome {
            name: "skip",
            result: CaseResult::Skipped("capability disabled".to_owned()),
        },
        CaseOutcome {
            name: "fail",
            result: CaseResult::Failed("broken".to_owned()),
        },
    ];
    assert_eq!(
        summarize_outcomes(&outcomes),
        RunSummary {
            passed: 1,
            xfailed: 1,
            skipped: 1,
            failed: 1,
        }
    );
}

#[test]
fn phase_expectations_distinguish_expected_failure_xfail_and_xpass() {
    let contract = SimulationContract {
        name: "suite/case::test::bluesim",
        assertions: &[],
        link_options: &[],
        simulation_options: &[],
        expectation: ExpectedOutcome::Fail {
            phase: SimulationPhase::Link,
            output: None,
        },
        output: OutputNormalization::Preserve,
        backend: SimulationBackend::Bluesim,
        vcd: None,
        requirement: Requirement::BluesimEnabled,
    };
    let work_dir = std::path::Path::new(".");
    let artifact_dir = std::path::Path::new(".");
    let failure = PhaseFailure::new(SimulationPhase::Link, "link failed".to_owned());
    assert_eq!(
        evaluate_contract_outcome(&contract, Err(failure), work_dir, artifact_dir).unwrap(),
        ContractRunOutcome::Passed
    );

    let xfail = SimulationContract {
        expectation: ExpectedOutcome::XFail {
            phase: SimulationPhase::Simulation,
            reason: "known simulator defect",
        },
        ..contract
    };
    let failure = PhaseFailure::new(SimulationPhase::Simulation, "simulator failed".to_owned());
    assert!(matches!(
        evaluate_contract_outcome(&xfail, Err(failure), work_dir, artifact_dir),
        Ok(ContractRunOutcome::XFailed(reason)) if reason.contains("known simulator defect")
    ));
    assert!(
        evaluate_contract_outcome(&xfail, Ok(()), work_dir, artifact_dir)
            .unwrap_err()
            .contains("XPASS")
    );
}

#[test]
fn output_normalization_sorts_lines_declaratively() {
    assert_eq!(
        normalize_contract_output(OutputNormalization::SortedLines, "second\nfirst\n"),
        "first\nsecond\n"
    );
}

#[test]
fn vcd_validation_parses_header_signals_and_complete_body() {
    let root = std::env::temp_dir().join(format!("bsc-rust-tests-vcd-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let valid = root.join("valid.vcd");
    std::fs::write(
        &valid,
        concat!(
            "$timescale 1 ns $end\n",
            "$scope module top $end\n",
            "$var wire 1 ! clock $end\n",
            "$upscope $end\n",
            "$enddefinitions $end\n",
            "#0\n0!\n#1\n1!\n"
        ),
    )
    .unwrap();
    validate_vcd(&valid).unwrap();

    let no_signals = root.join("no-signals.vcd");
    std::fs::write(&no_signals, "$enddefinitions $end\n").unwrap();
    assert!(validate_vcd(&no_signals)
        .unwrap_err()
        .contains("declares no signals"));

    let invalid = root.join("invalid.vcd");
    std::fs::write(
        &invalid,
        "$scope module top $end\n$var wire 1 ! clock $end\n$upscope $end\n$enddefinitions $end\n#broken\n",
    )
    .unwrap();
    assert!(validate_vcd(&invalid)
        .unwrap_err()
        .contains("parse VCD body"));
}

#[test]
fn diagnostic_count_matches_tcl_line_regexp_shape() {
    let output = concat!(
        "Error: \"file\", line 1, column 2: (P0070)\n",
        "  details (P0070)\n",
        "prefix Error: Unknown position: (P0070)\r\n",
        "Error:(P0070)\n",
        "Error: x (OTHER)\n",
        "Warning: x (P0070)\n"
    );
    assert_eq!(count_diagnostics(output, DiagnosticKind::Error, "P0070"), 2);
    assert_eq!(
        count_diagnostics(output, DiagnosticKind::Warning, "P0070"),
        1
    );
}

#[test]
fn text_assertions_cover_fixed_string_regex_and_diagnostic_checks() {
    let text = concat!(
        "alpha\n",
        "argument 2 first\n",
        "argument 2 second\n",
        "Error: x (G0055)\n"
    );
    let assertions = [
        TextAssertion::Contains { text: "alpha" },
        TextAssertion::DoesNotContain { text: "omega" },
        TextAssertion::LineCount {
            text: "argument 2",
            count: 2,
        },
        TextAssertion::Regex {
            pattern: r"^argument [0-9]",
        },
        TextAssertion::RegexDoesNotMatch {
            pattern: r"^omega$",
        },
        TextAssertion::RegexCount {
            pattern: r"^argument [0-9]",
            count: 2,
        },
        TextAssertion::DiagnosticCount {
            kind: DiagnosticKind::Error,
            tag: "G0055",
            count: 1,
        },
    ];
    for assertion in assertions {
        check_text_assertion(text, assertion).unwrap();
    }
    assert!(check_text_assertion(
        text,
        TextAssertion::LineCount {
            text: "argument 2",
            count: 1,
        }
    )
    .is_err());
}

#[test]
fn regex_assertions_normalize_crlf_and_lone_cr_newlines() {
    let text = "alpha\r\nforbidden\romega\r\nforbidden\r";
    check_text_assertion(
        text,
        TextAssertion::Regex {
            pattern: "^alpha$\n^forbidden$\n^omega$",
        },
    )
    .unwrap();
    check_text_assertion(
        text,
        TextAssertion::RegexDoesNotMatch {
            pattern: r"^missing$",
        },
    )
    .unwrap();

    let error = check_text_assertion(
        text,
        TextAssertion::RegexDoesNotMatch {
            pattern: r"^forbidden$",
        },
    )
    .unwrap_err();
    assert!(error.contains("found 2 matches"), "{error}");
}

#[test]
fn artifact_validation_precompiles_text_regexes() {
    for assertion in [
        TextAssertion::Regex { pattern: "(" },
        TextAssertion::RegexDoesNotMatch { pattern: "(" },
        TextAssertion::RegexCount {
            pattern: "(",
            count: 0,
        },
    ] {
        let error = validate_artifact_assertions(
            &[ArtifactAssertion::Text {
                path: "output.txt",
                assertion,
            }],
            &[],
            "test",
        )
        .unwrap_err();
        assert!(error.contains("invalid multiline regex"), "{error}");
    }
}

#[test]
fn system_verilog_artifact_assertion_parses_valid_verilog_and_rejects_invalid_syntax() {
    let root = std::env::temp_dir().join(format!(
        "bsc-rust-system-verilog-artifact-{}",
        crate::current_run_id()
    ));
    let work = root.join("work");
    let artifacts = root.join("artifacts");
    std::fs::create_dir_all(&work).unwrap();
    std::fs::create_dir_all(&artifacts).unwrap();

    let assertion = ArtifactAssertion::ParsesAsSystemVerilog { path: "smoke.v" };
    assert_eq!(assertion.actual_path(), "smoke.v");
    assert_eq!(assertion.expected_path(), None);
    validate_artifact_assertions(&[assertion], &[], "parser smoke test").unwrap();

    std::fs::write(
        work.join("smoke.v"),
        "`include \"intentionally-missing.svh\"\nmodule smoke; wire value; endmodule\n",
    )
    .unwrap();
    check_artifact_assertions(&[assertion], &work, &artifacts, "parser smoke test").unwrap();

    std::fs::write(
        work.join("smoke.v"),
        "module broken; this is not verilog; endmodule\n",
    )
    .unwrap();
    let error = check_artifact_assertions(&[assertion], &work, &artifacts, "parser smoke test")
        .unwrap_err();
    assert!(error.contains("smoke.v"), "{error}");
    assert!(error.contains("parser smoke test"), "{error}");
    assert!(error.contains("Parse error"), "{error}");

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn system_verilog_artifact_assertion_uses_shared_path_safety_validation() {
    let assertion = ArtifactAssertion::ParsesAsSystemVerilog {
        path: "../outside.sv",
    };
    let error = validate_artifact_assertions(&[assertion], &[], "parser safety test").unwrap_err();
    assert!(error.contains("unsafe artifact path"), "{error}");
    assert!(error.contains("../outside.sv"), "{error}");
}

#[test]
fn artifact_assertions_compare_exact_golden_and_verilog_outputs() {
    let root = std::env::temp_dir().join(format!("bsc-rust-artifact-{}", crate::current_run_id()));
    let work = root.join("work");
    let artifacts = root.join("artifacts");
    std::fs::create_dir_all(&work).unwrap();
    std::fs::create_dir_all(&artifacts).unwrap();
    std::fs::write(work.join("actual.bin"), [0, 1, 2]).unwrap();
    std::fs::write(work.join("expected.bin"), [0, 1, 2]).unwrap();
    std::fs::write(work.join("actual.txt"), "alpha\tbeta\nSystemC noise\n").unwrap();
    std::fs::write(work.join("expected.txt"), "alpha  beta\n").unwrap();
    std::fs::write(
        work.join("actual.v"),
        "// Bluespec Compiler build A\nwire __h123;\n",
    )
    .unwrap();
    std::fs::write(
        work.join("expected.v"),
        "// Bluespec Compiler build B\nwire __h456;\n",
    )
    .unwrap();

    let assertions = [
        ArtifactAssertion::Matches {
            actual: "actual.bin",
            expected: "expected.bin",
            normalization: ArtifactNormalization::Exact,
        },
        ArtifactAssertion::Matches {
            actual: "actual.txt",
            expected: "expected.txt",
            normalization: ArtifactNormalization::GoldenOutput,
        },
        ArtifactAssertion::Matches {
            actual: "actual.v",
            expected: "expected.v",
            normalization: ArtifactNormalization::Verilog,
        },
    ];
    let fixtures = ["expected.bin", "expected.txt", "expected.v"];
    validate_artifact_assertions(&assertions, &fixtures, "test").unwrap();
    check_artifact_assertions(&assertions, &work, &artifacts, "test").unwrap();

    std::fs::write(work.join("actual.bin"), [3, 4]).unwrap();
    assert!(check_artifact_assertions(&assertions, &work, &artifacts, "test").is_err());
    assert!(artifacts.join("artifact-0.diff").is_file());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn golden_output_uses_diff_b_and_line_filters() {
    let expected = "alpha  beta\nSystemC banner\ndumpfile parameter ignored\nlast\tvalue\n";
    let actual = "alpha\tbeta\ncompiling ./Dependency.bs\nlast value   \r\n";
    assert_eq!(
        normalize_golden_output(expected),
        normalize_golden_output(actual)
    );
}

#[test]
fn golden_output_normalizes_windows_scientific_exponents() {
    let expected = "9.70e+01 -9.400000e+01 2.00204E-08\n";
    let windows = "9.70e+001 -9.400000e+001 2.00204E-008\n";
    assert_eq!(
        normalize_golden_output(expected),
        normalize_golden_output(windows)
    );
}

#[test]
fn golden_output_ignores_a_missing_final_newline() {
    assert_eq!(
        normalize_golden_output("same output\n"),
        normalize_golden_output("same output")
    );
}

#[test]
fn cli_parses_filter_exact_and_thread_count() {
    let name = "bsc.bugs/bluespec_inc/b1493::Bug1493_Bad.bsv";
    let options = parse_cli([name, "--exact", "--test-threads=3"]).unwrap();
    assert!(options.exact);
    assert!(options.bluesim_enabled);
    assert!(options.verilog_enabled);
    assert_eq!(options.test_threads, 3);
    assert_eq!(options.filter.as_deref(), Some(name));
    assert_eq!(select_plan(&options).contract_count(), 1);
}

#[test]
fn cli_list_and_substring_selection() {
    let options = parse_cli([
        "--list",
        "b1493",
        "--no-bluesim",
        "--no-verilog",
        "--test-threads",
        "2",
    ])
    .unwrap();
    assert!(options.list);
    assert!(!options.bluesim_enabled);
    assert!(!options.verilog_enabled);
    let plan = select_plan(&options);
    let names = plan.contract_names().collect::<Vec<_>>();
    assert_eq!(
        names,
        [
            "bsc.bugs/bluespec_inc/b1493::Bug1493.bsv",
            "bsc.bugs/bluespec_inc/b1493::Bug1493_Bad.bsv"
        ]
    );
}

#[test]
fn cli_rejects_bad_thread_counts_and_multiple_filters() {
    assert!(parse_cli(["--test-threads", "0"]).is_err());
    assert!(parse_cli(["one", "two"]).is_err());
    assert!(parse_cli(["--unknown"]).is_err());
}

#[test]
fn fixed_worker_queue_processes_every_item_once() {
    let results = run_fixed_queue((0..20).collect(), 4, |value| value * value);
    let actual: BTreeSet<_> = results.into_iter().collect();
    let expected: BTreeSet<_> = (0..20).map(|value| value * value).collect();
    assert_eq!(actual, expected);
}

#[test]
fn run_paths_are_scoped_by_run_id() {
    let paths = RunPaths::new(Path::new("project"), "123-456");
    assert!(paths.work_root.ends_with("upstream/123-456"));
    assert!(paths.artifact_root.ends_with("upstream/123-456"));
}
