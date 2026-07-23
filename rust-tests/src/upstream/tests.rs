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

    let phase_one_names = [
        "b600::Bug600.bsv",
        "b267::Bug267.bs",
        "b1040::Bug1040.bsv",
        "b417::Bug417.bsv",
        "b492::Bug492_1.bs",
        "b1586::Bug1586.bsv",
        "b269::Bug269.bsv",
        "b1493::Bug1493.bsv",
        "b1493::Bug1493_Bad.bsv",
    ];
    assert!(phase_one_names.into_iter().all(|name| names.contains(name)));
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
}

#[test]
fn vtest_policy_defaults_enabled_and_zero_disables_verilog() {
    let cases = compile_cases();
    let default_policy = RunnerPolicy::from_vtest(None);
    assert!(default_policy.verilog_enabled);
    assert!(cases
        .iter()
        .all(|case| default_policy.skip_reason(case.requirement).is_none()));

    let disabled = RunnerPolicy::from_vtest(Some(std::ffi::OsStr::new("0")));
    assert!(!disabled.verilog_enabled);
    let skipped: Vec<_> = cases
        .iter()
        .filter(|case| disabled.skip_reason(case.requirement).is_some())
        .collect();
    let verilog_cases = cases
        .iter()
        .filter(|case| matches!(case.mode, CompileMode::Verilog { .. }))
        .count();
    assert_eq!(skipped.len(), verilog_cases);
    assert!(skipped
        .iter()
        .all(|case| matches!(case.mode, CompileMode::Verilog { .. })));

    assert!(RunnerPolicy::from_vtest(Some(std::ffi::OsStr::new("1"))).verilog_enabled);
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

    let all = all_cases();
    assert_eq!(all.len(), compile_cases().len() + contract_count);
    let all_names: BTreeSet<_> = all.iter().map(|case| case.name()).collect();
    assert_eq!(
        all_names.len(),
        all.len(),
        "all upstream contract names must be unique"
    );
}

#[test]
fn work_items_follow_declared_generation_scenarios() {
    let all = all_cases();
    let aes = all
        .iter()
        .copied()
        .filter(|case| case.name().contains("bsc.bsv_examples/AES::Aes_TB::"))
        .collect::<Vec<_>>();
    assert_eq!(aes.len(), 2);
    let work = build_work_items(aes);
    assert_eq!(work.len(), 1);
    match &work[0] {
        WorkItem::Simulation {
            scenario,
            contracts,
        } => {
            assert_eq!(contracts.len(), 2);
            assert_eq!(scenario.generation, GenerationStrategy::SharedElaboration);
            assert_eq!(scenario.resource, ResourceClass::Heavy);
            assert_eq!(scenario.timeout, crate::BSC_HEAVY_TIMEOUT);
        }
        WorkItem::Compile(_) => panic!("AES contracts were not grouped by their scenario"),
    }

    let positive_reset = all
        .iter()
        .copied()
        .filter(|case| {
            case.name()
                .contains("bsc.verilog/positivereset/SyncReset::RstTest::")
        })
        .collect::<Vec<_>>();
    assert_eq!(positive_reset.len(), 2);
    assert_eq!(build_work_items(positive_reset).len(), 2);
}

#[test]
fn ctest_and_vtest_capabilities_skip_their_simulation_backends() {
    let cases = all_cases();
    let no_bluesim = RunnerPolicy::from_environment(
        Some(std::ffi::OsStr::new("0")),
        Some(std::ffi::OsStr::new("1")),
    )
    .with_iverilog_major(Some(13));
    let skipped_without_bluesim = cases
        .iter()
        .filter(|case| no_bluesim.skip_reason(case.requirement()).is_some())
        .count();
    let bluesim_cases = cases
        .iter()
        .filter(|case| case.requirement() == Requirement::BluesimEnabled)
        .count();
    assert_eq!(skipped_without_bluesim, bluesim_cases);

    let no_verilog = RunnerPolicy::from_environment(
        Some(std::ffi::OsStr::new("1")),
        Some(std::ffi::OsStr::new("0")),
    );
    let skipped_without_verilog = cases
        .iter()
        .filter(|case| no_verilog.skip_reason(case.requirement()).is_some())
        .count();
    let verilog_cases = cases
        .iter()
        .filter(|case| {
            matches!(
                case.requirement(),
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
fn iverilog_output_filter_matches_legacy_noise_rules() {
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
fn outcome_summary_counts_skips_without_turning_them_into_failures() {
    let outcomes = [
        CaseOutcome {
            name: "pass",
            result: CaseResult::Passed,
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
            skipped: 1,
            failed: 1,
        }
    );
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
fn legacy_golden_uses_diff_b_and_line_filters() {
    let expected = "alpha  beta\nSystemC banner\ndumpfile parameter ignored\nlast\tvalue\n";
    let actual = "alpha\tbeta\ncompiling ./Dependency.bs\nlast value   \r\n";
    assert_eq!(
        normalize_legacy_golden(expected),
        normalize_legacy_golden(actual)
    );
}

#[test]
fn legacy_golden_normalizes_windows_scientific_exponents() {
    let expected = "9.70e+01 -9.400000e+01 2.00204E-08\n";
    let windows = "9.70e+001 -9.400000e+001 2.00204E-008\n";
    assert_eq!(
        normalize_legacy_golden(expected),
        normalize_legacy_golden(windows)
    );
}

#[test]
fn legacy_golden_ignores_a_missing_final_newline() {
    assert_eq!(
        normalize_legacy_golden("same output\n"),
        normalize_legacy_golden("same output")
    );
}

#[test]
fn cli_parses_filter_exact_and_thread_count() {
    let options = parse_cli(["b1493::Bug1493_Bad.bsv", "--exact", "--test-threads=3"]).unwrap();
    assert!(options.exact);
    assert_eq!(options.test_threads, 3);
    assert_eq!(options.filter.as_deref(), Some("b1493::Bug1493_Bad.bsv"));
    let cases = all_cases();
    assert_eq!(select_cases(&cases, &options).len(), 1);
}

#[test]
fn cli_list_and_substring_selection() {
    let options = parse_cli(["--list", "b1493", "--test-threads", "2"]).unwrap();
    assert!(options.list);
    let cases = all_cases();
    let names: Vec<&str> = select_cases(&cases, &options)
        .into_iter()
        .map(|case| case.name())
        .collect();
    assert_eq!(names, ["b1493::Bug1493.bsv", "b1493::Bug1493_Bad.bsv"]);
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
