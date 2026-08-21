#![forbid(unsafe_code)]

use regex::Regex;
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

pub const TEST_PLAN_SCHEMA_VERSION: u32 = 50;
pub const TEST_PLAN_INDEX_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TestPlan {
    pub schema_version: u32,
    pub id: String,
    pub origin: Origin,
    pub status: PlanStatus,
    pub fixture_dir: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fixtures: Vec<Fixture>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scenarios: Vec<Scenario>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<ImportDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Origin {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    Complete,
    Disabled,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Fixture {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub sha256: String,
    pub role: FixtureRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FixtureRole {
    Source,
    Golden,
    Script,
    CommandFile,
    Data,
    BuildInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Scenario {
    pub id: String,
    pub resource: ResourceClass,
    pub fixtures: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<Requirement>,
    /// Static arguments appended to the inherited BSC_OPTIONS for BSC child processes only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bsc_options_append: Option<String>,
    pub timeouts: Timeouts,
    pub stages: Vec<Stage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResourceClass {
    Normal,
    Heavy,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Requirement {
    Bluesim,
    Verilog,
    Frontend,
    Icarus,
    Bluetcl,
    BluetclPackage(BluetclPackage),
    SystemC,
    ShowRules,
    NonWindows,
    InternalChecks,
    PosixUnreadability,
    Darwin,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum BluetclPackage {
    InstSynth,
    ExpandPorts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BluetclInstalledScript {
    ExpandPorts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Timeouts {
    pub generation_seconds: u64,
    pub link_seconds: u64,
    pub simulation_seconds: u64,
    pub assertion_seconds: u64,
}

impl Default for Timeouts {
    fn default() -> Self {
        Self {
            generation_seconds: 300,
            link_seconds: 300,
            simulation_seconds: 300,
            assertion_seconds: 30,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Stage {
    pub id: String,
    pub operations: Vec<OperationRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OperationRecord {
    #[serde(flatten)]
    pub action: Action,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<Requirement>,
    pub artifacts: ArtifactContract,
    #[serde(default, skip_serializing_if = "OperationExpectation::is_required")]
    pub expectation: OperationExpectation,
    pub provenance: Provenance,
}

impl OperationRecord {
    pub fn new(action: Action, expectation: OperationExpectation, provenance: Provenance) -> Self {
        let artifacts = ArtifactContract::for_action(&action);
        Self {
            action,
            requires: Vec::new(),
            artifacts,
            expectation,
            provenance,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactContract {
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub output_alternatives: Vec<Vec<String>>,
    pub directories: Vec<String>,
    pub removes: Vec<String>,
}

fn path_in_working_directory(working_directory: Option<&str>, path: &str) -> String {
    working_directory.map_or_else(
        || path.to_owned(),
        |directory| format!("{directory}/{path}"),
    )
}

impl ArtifactContract {
    pub fn for_action(action: &Action) -> Self {
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        let mut output_alternatives = Vec::new();
        let mut directories = Vec::new();
        let mut removes = Vec::new();
        match action {
            Action::BscCompile {
                source,
                working_directory,
                stdout,
                ..
            } => {
                inputs.push(path_in_working_directory(
                    working_directory.as_deref(),
                    source,
                ));
                outputs.push(path_in_working_directory(
                    working_directory.as_deref(),
                    stdout,
                ));
            }
            Action::BscOptions { stdout, .. } | Action::BscFlagPreflight { stdout, .. } => {
                outputs.push(stdout.clone())
            }
            Action::BluetclRun {
                invocation,
                artifact_inputs,
                artifact_outputs,
                stdout,
                ..
            } => {
                if let Some(script) = invocation.fixture_script() {
                    inputs.push(script.to_owned());
                }
                inputs.extend(artifact_inputs.iter().cloned());
                outputs.push(stdout.clone());
                outputs.extend(artifact_outputs.iter().cloned());
            }
            Action::MakeTestData => {
                inputs.extend(["Makefile.data".to_owned(), "dumper.c".to_owned()]);
                outputs.extend([
                    "testa.dat".to_owned(),
                    "testm.dat".to_owned(),
                    "testmac.dat".to_owned(),
                    "testa64.dat".to_owned(),
                    "testm64.dat".to_owned(),
                    "testmac64.dat".to_owned(),
                ]);
                output_alternatives.push(vec!["dumper".to_owned(), "dumper.exe".to_owned()]);
            }
            Action::InterraOperatorVectors { suite } => {
                inputs.extend([
                    "generate/gen.pl".to_owned(),
                    "generate/sort.pl".to_owned(),
                    "generate/top_code".to_owned(),
                    "generate/bot_code".to_owned(),
                ]);
                outputs.extend([
                    format!("generate/{}", suite.generated_verilog()),
                    "generate/a.out".to_owned(),
                    "generate/vectors".to_owned(),
                    "generate/Vectors.bsv".to_owned(),
                    "Vectors.bsv".to_owned(),
                ]);
            }
            Action::Bsc2Bsv { source, stdout } => {
                inputs.push(source.clone());
                outputs.push(stdout.clone());
            }
            Action::BscParsePretty {
                source,
                pretty_output,
                ..
            } => {
                inputs.push(source.clone());
                outputs.extend([
                    pretty_output.clone(),
                    format!("{source}.bsc-out"),
                    format!("{pretty_output}.bsc-out"),
                ]);
            }
            Action::DumpIntermediate { input, output, .. } => {
                inputs.push(input.clone());
                outputs.push(output.clone());
            }
            Action::RenderGolden {
                template, output, ..
            }
            | Action::M4CurdirRender { template, output } => {
                inputs.push(template.clone());
                outputs.push(output.clone());
            }
            Action::TextNormalize {
                source,
                destination,
                ..
            } => {
                inputs.push(source.clone());
                outputs.push(destination.clone());
            }
            Action::VerilogFilter { path, profiles, .. } => {
                inputs.push(path.clone());
                inputs.extend(
                    profiles
                        .iter()
                        .filter_map(|profile| profile.fixture_path().map(str::to_owned)),
                );
                outputs.push(path.clone());
            }
            Action::BscSimirExport { top, output } => {
                inputs.push(format!("{top}.ba"));
                outputs.push(output.clone());
            }
            Action::SimirM0Step { model, stdout, .. }
            | Action::SimirM2Run { model, stdout, .. }
            | Action::SimirM3Run { model, stdout, .. } => {
                inputs.push(model.clone());
                outputs.push(stdout.clone());
            }
            Action::BscGenerate {
                source,
                mode,
                module,
                args,
            } => {
                inputs.push(source.clone());
                outputs.push(mode.compiler_output_path(source));
                outputs.extend(generation_package_artifacts(source, args));
                outputs.extend(generation_static_dump_artifacts(args, module.as_deref()));
                if let Some(module) = module {
                    match mode {
                        SimulationGenerationMode::Bluesim => {
                            outputs.push(format!("{module}.ba"));
                        }
                        SimulationGenerationMode::Verilog => {
                            outputs.push(format!("{module}.v"));
                        }
                        SimulationGenerationMode::SharedElaboration => {
                            outputs.extend([format!("{module}.ba"), format!("{module}.v")])
                        }
                    }
                }
            }
            Action::CObjectBuild {
                source,
                makefile,
                output,
            } => {
                inputs.extend([source.clone(), makefile.clone()]);
                outputs.push(output.clone());
            }
            Action::BscLink {
                backend,
                objects,
                top,
                mode,
                args,
                expected_exit,
                missing_objects,
                simulator,
            } => {
                inputs.extend(
                    objects
                        .iter()
                        .filter(|object| !missing_objects.contains(object))
                        .map(|object| {
                            if std::path::Path::new(object).extension().is_some() {
                                object.clone()
                            } else {
                                let extension = match backend {
                                    SimulationBackend::Bluesim => "ba",
                                    SimulationBackend::Icarus => "v",
                                };
                                format!("{object}.{extension}")
                            }
                        }),
                );
                if *mode == BscLinkMode::Standard {
                    inputs.extend(link_native_inputs(args));
                }
                if *backend == SimulationBackend::Icarus && *mode == BscLinkMode::Standard {
                    inputs.extend(link_file_inputs(args));
                }
                match mode {
                    BscLinkMode::Standard => {
                        outputs.push(match backend {
                            SimulationBackend::Bluesim => format!("{top}.bsc-ccomp-out"),
                            SimulationBackend::Icarus => format!("{top}.bsc-vcomp-out"),
                        });
                    }
                    BscLinkMode::NoMain => outputs.push(format!("{top}.bsc-vcomp-out")),
                }
                if *expected_exit == ExpectedExit::Success && simulator.produces_executable() {
                    match backend {
                        SimulationBackend::Icarus => {
                            outputs.push(simulation_executable_artifact(*backend, top));
                        }
                        SimulationBackend::Bluesim => {
                            let executable = simulation_executable_artifact(*backend, top);
                            output_alternatives
                                .push(vec![executable.clone(), format!("{executable}.exe")]);
                        }
                    }
                }
            }
            Action::SimulationRun {
                backend,
                executable,
                args,
                stdout,
                vcd,
                ..
            } => {
                inputs.push(simulation_executable_artifact(*backend, executable));
                inputs.extend(simulation_file_inputs(args).into_iter().map(str::to_owned));
                outputs.push(stdout.clone());
                outputs.extend(simulation_vcd_outputs(args));
                outputs.extend(vcd.iter().cloned());
            }
            Action::ShowRules {
                input,
                output,
                design_inputs,
                stdout,
                ..
            } => {
                inputs.push(input.clone());
                inputs.extend(design_inputs.iter().cloned());
                outputs.extend([output.clone(), stdout.clone()]);
            }
            Action::BscSystemcLink {
                objects,
                top,
                expected_exit,
            } => {
                inputs.extend(objects.iter().cloned());
                outputs.push(format!("{top}.bsc-ccomp-out"));
                if *expected_exit == ExpectedExit::Success {
                    outputs.extend([
                        format!("{top}.o"),
                        format!("{top}_systemc.o"),
                        format!("model_{top}.o"),
                    ]);
                }
            }
            Action::SystemcCxxLink {
                executable,
                sources,
                top_modules,
                other_modules,
                ..
            } => {
                inputs.extend(sources.iter().cloned());
                inputs.extend(
                    top_modules
                        .iter()
                        .chain(other_modules)
                        .flat_map(|module| [format!("{module}.o"), format!("{module}_systemc.o")]),
                );
                inputs.extend(top_modules.iter().map(|module| format!("model_{module}.o")));
                outputs.extend([
                    format!("{executable}.syscexe"),
                    format!("{executable}.cxx-comp-out"),
                ]);
            }
            Action::SystemcRun {
                executable, stdout, ..
            } => {
                inputs.push(format!("{executable}.syscexe"));
                outputs.extend([stdout.clone(), format!("{executable}.raw.out")]);
            }
            Action::FsCopy {
                source,
                destination,
            } => {
                inputs.push(source.clone());
                outputs.push(destination.clone());
            }
            Action::FsCopyReplace {
                source,
                destination,
            } => {
                inputs.extend([source.clone(), destination.clone()]);
                outputs.push(destination.clone());
            }
            Action::FsRewriteDarwinCppIncludePath {
                source,
                destination,
            } => {
                inputs.push(source.clone());
                outputs.push(destination.clone());
            }
            Action::FsMove {
                source,
                destination,
            } => {
                inputs.push(source.clone());
                outputs.push(destination.clone());
                removes.push(source.clone());
            }
            Action::FsMoveReplace {
                source,
                destination,
            } => {
                inputs.extend([source.clone(), destination.clone()]);
                outputs.push(destination.clone());
                removes.push(source.clone());
            }
            Action::FsRemove { path } => {
                inputs.push(path.clone());
                removes.push(path.clone())
            }
            Action::FsEnsureAbsent { path } | Action::FsEnsureDirectoryAbsent { path } => {
                removes.push(path.clone())
            }
            Action::FsTouch { path } => {
                inputs.push(path.clone());
                outputs.push(path.clone());
            }
            Action::FsTouchCreate { path, .. } => outputs.push(path.clone()),
            Action::FsRemoveUserRead { path } => {
                inputs.push(path.clone());
                outputs.push(path.clone());
            }
            Action::VcdCheck { path, .. }
            | Action::AssertExists { path }
            | Action::AssertTextContains { path, .. }
            | Action::AssertTextAbsent { path, .. }
            | Action::AssertRegex { path, .. }
            | Action::AssertRegexAbsent { path, .. }
            | Action::AssertTextCount { path, .. }
            | Action::AssertRegexCount { path, .. }
            | Action::AssertDiagnosticCount { path, .. }
            | Action::AssertVcdValid { path }
            | Action::AssertVcdValidIfPresent { path } => inputs.push(path.clone()),
            Action::AssertGoldenMissingXfail { actual, .. } => inputs.push(actual.clone()),
            Action::AssertGolden { actual, expected }
            | Action::AssertGoldenNative { actual, expected }
            | Action::AssertGoldenNormalized {
                actual, expected, ..
            }
            | Action::AssertGoldenSortedLines { actual, expected }
            | Action::AssertGoldenXfail {
                actual, expected, ..
            }
            | Action::AssertVerilog { actual, expected }
            | Action::AssertVcd { actual, expected } => {
                inputs.extend([actual.clone(), expected.clone()]);
            }
            Action::AssertGoldenAny { actual, expected } => {
                inputs.push(actual.clone());
                inputs.extend(expected.iter().cloned());
            }
            Action::FsMkdir { path } | Action::FsCreateDirAll { path } => {
                directories.push(path.clone())
            }
            Action::Delay { .. } => {}
        }
        let unique = |paths: &mut Vec<String>| {
            let mut seen = BTreeSet::new();
            paths.retain(|path| seen.insert(path.to_ascii_lowercase()));
        };
        unique(&mut inputs);
        unique(&mut outputs);
        unique(&mut directories);
        unique(&mut removes);
        for alternatives in &mut output_alternatives {
            unique(alternatives);
        }
        Self {
            inputs,
            outputs,
            output_alternatives,
            directories,
            removes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OperationExpectation {
    #[default]
    Required,
    Xfail {
        reason: String,
    },
}

impl OperationExpectation {
    pub fn is_required(&self) -> bool {
        matches!(self, Self::Required)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all_fields = "camelCase", deny_unknown_fields)]
pub enum Action {
    #[serde(rename = "bsc.compile")]
    BscCompile {
        source: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        working_directory: Option<String>,
        mode: BscCompileMode,
        #[serde(skip_serializing_if = "Option::is_none")]
        module: Option<String>,
        args: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        absolute_import_paths: Vec<String>,
        dependency_mode: DependencyMode,
        expected_exit: ExpectedExit,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        unexpected_success_forbidden_regex: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        environment: Option<BscCompileEnvironment>,
        stdout: String,
    },
    #[serde(rename = "bsc.options")]
    BscOptions {
        args: Vec<String>,
        #[serde(default, skip_serializing_if = "ExpectedExit::is_success")]
        expected_exit: ExpectedExit,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bsc_options_prepend: Option<String>,
        stdout: String,
    },
    #[serde(rename = "bsc.flag_preflight")]
    BscFlagPreflight {
        mode: BscFlagPreflightMode,
        input: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        top: Option<String>,
        unspecified_to: UndeterminedValue,
        stdout: String,
    },
    #[serde(rename = "bluetcl.run")]
    BluetclRun {
        invocation: BluetclInvocation,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        working_directory: Option<String>,
        artifact_inputs: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        artifact_outputs: Vec<String>,
        #[serde(default, skip_serializing_if = "ExpectedExit::is_success")]
        expected_exit: ExpectedExit,
        stdout: String,
    },
    #[serde(rename = "upstream.make_test_data")]
    MakeTestData,
    #[serde(rename = "fixture.interra_operator_vectors")]
    InterraOperatorVectors { suite: InterraOperatorSuite },
    #[serde(rename = "golden.render")]
    RenderGolden {
        template: String,
        output: String,
        replacement: GoldenReplacement,
    },
    #[serde(rename = "template.m4_curdir")]
    M4CurdirRender { template: String, output: String },
    #[serde(rename = "text.normalize")]
    TextNormalize {
        source: String,
        destination: String,
        transform: TextNormalization,
    },
    #[serde(rename = "verilog.filter")]
    VerilogFilter {
        path: String,
        profiles: Vec<VerilogFilterProfile>,
        #[serde(default, skip_serializing_if = "ExpectedExit::is_success")]
        expected_exit: ExpectedExit,
    },
    #[serde(rename = "bsc.generate")]
    BscGenerate {
        source: String,
        mode: SimulationGenerationMode,
        #[serde(skip_serializing_if = "Option::is_none")]
        module: Option<String>,
        args: Vec<String>,
    },
    /// Export the optimized legacy Bluesim schedule as the deliberately narrow M0 SimIR subset.
    #[serde(rename = "bsc.simir_export")]
    BscSimirExport { top: String, output: String },
    /// Execute a generated M0 SimIR model using the Rust Bluesim library for a fixed number of cycles.
    #[serde(rename = "simir.m0_step")]
    SimirM0Step {
        model: String,
        cycles: u64,
        stdout: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expected_finish: Option<i32>,
    },
    /// Run a generated M2 SimIR model in-process until it finishes or reaches its event limit.
    #[serde(rename = "simir.m2_run")]
    SimirM2Run {
        model: String,
        max_events: u64,
        expected_finish: i32,
        expected_time: u64,
        stdout: String,
    },
    /// Run a generated M3 SimIR model in-process until it finishes or reaches its event limit.
    #[serde(rename = "simir.m3_run")]
    SimirM3Run {
        model: String,
        max_events: u64,
        expected_finish: i32,
        expected_time: u64,
        stdout: String,
    },
    #[serde(rename = "c.compile_object")]
    CObjectBuild {
        source: String,
        makefile: String,
        output: String,
    },
    #[serde(rename = "bsc.link")]
    BscLink {
        backend: SimulationBackend,
        #[serde(default, skip_serializing_if = "BscLinkMode::is_standard")]
        mode: BscLinkMode,
        objects: Vec<String>,
        top: String,
        args: Vec<String>,
        #[serde(default, skip_serializing_if = "ExpectedExit::is_success")]
        expected_exit: ExpectedExit,
        #[serde(default, skip_serializing_if = "IcarusSimulatorSelector::is_default")]
        simulator: IcarusSimulatorSelector,
        /// Objects that are deliberately passed to the linker even though
        /// nothing produces them, because the plan asserts the resulting
        /// missing-module (G0084) link failure. They stay in the argv but are
        /// exempt from the produced-or-fixture input requirement.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        missing_objects: Vec<String>,
    },
    #[serde(rename = "bsc.systemc_link")]
    BscSystemcLink {
        objects: Vec<String>,
        top: String,
        #[serde(default, skip_serializing_if = "ExpectedExit::is_success")]
        expected_exit: ExpectedExit,
    },
    #[serde(rename = "systemc.cxx_link")]
    SystemcCxxLink {
        executable: String,
        sources: Vec<String>,
        top_modules: Vec<String>,
        other_modules: Vec<String>,
        defines: Vec<String>,
    },
    #[serde(rename = "systemc.run")]
    SystemcRun {
        executable: String,
        stdout: String,
        sort_output: bool,
    },
    #[serde(rename = "internal.bsc2bsv")]
    Bsc2Bsv { source: String, stdout: String },
    #[serde(rename = "bsc.parse_pretty_roundtrip")]
    BscParsePretty {
        source: String,
        args: Vec<String>,
        pretty_output: String,
    },
    #[serde(rename = "internal.dump")]
    DumpIntermediate {
        input: String,
        output: String,
        view: IntermediateDumpView,
    },
    #[serde(rename = "simulation.run")]
    SimulationRun {
        backend: SimulationBackend,
        executable: String,
        args: Vec<String>,
        stdout: String,
        #[serde(default, skip_serializing_if = "ExpectedExitSet::is_success")]
        expected_exits: ExpectedExitSet,
        #[serde(skip_serializing_if = "Option::is_none")]
        vcd: Option<String>,
    },
    #[serde(rename = "vcd.showrules")]
    ShowRules {
        top: String,
        input: String,
        output: String,
        design_inputs: Vec<String>,
        stdout: String,
    },
    #[serde(rename = "vcd.check")]
    VcdCheck {
        path: String,
        checks: Vec<String>,
        #[serde(default, skip_serializing_if = "ExpectedExit::is_success")]
        expected_exit: ExpectedExit,
    },
    #[serde(rename = "fs.copy")]
    FsCopy { source: String, destination: String },
    #[serde(rename = "fs.copy_replace")]
    FsCopyReplace { source: String, destination: String },
    #[serde(rename = "fs.rewrite_darwin_cpp_include_path")]
    FsRewriteDarwinCppIncludePath { source: String, destination: String },
    #[serde(rename = "fs.move")]
    FsMove { source: String, destination: String },
    #[serde(rename = "fs.move_replace")]
    FsMoveReplace { source: String, destination: String },
    #[serde(rename = "fs.remove")]
    FsRemove { path: String },
    #[serde(rename = "fs.ensure_absent")]
    FsEnsureAbsent { path: String },
    #[serde(rename = "fs.ensure_dir_absent")]
    FsEnsureDirectoryAbsent { path: String },
    #[serde(rename = "fs.mkdir")]
    FsMkdir { path: String },
    #[serde(rename = "fs.create_dir_all")]
    FsCreateDirAll { path: String },
    #[serde(rename = "fs.touch")]
    FsTouch { path: String },
    #[serde(rename = "fs.touch_create")]
    FsTouchCreate {
        path: String,
        delay_milliseconds: u64,
    },
    #[serde(rename = "fs.remove_user_read")]
    FsRemoveUserRead { path: String },
    #[serde(rename = "time.delay")]
    Delay { milliseconds: u64 },
    #[serde(rename = "assert.exists")]
    AssertExists { path: String },
    #[serde(rename = "assert.text_contains")]
    AssertTextContains { path: String, text: String },
    #[serde(rename = "assert.text_absent")]
    AssertTextAbsent { path: String, text: String },
    #[serde(rename = "assert.regex")]
    AssertRegex { path: String, pattern: String },
    #[serde(rename = "assert.regex_absent")]
    AssertRegexAbsent { path: String, pattern: String },
    #[serde(rename = "assert.text_count")]
    AssertTextCount {
        path: String,
        text: String,
        count: usize,
    },
    #[serde(rename = "assert.regex_count")]
    AssertRegexCount {
        path: String,
        pattern: String,
        count: usize,
    },
    #[serde(rename = "assert.diagnostic_count")]
    AssertDiagnosticCount {
        path: String,
        kind: DiagnosticKind,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        code: Option<String>,
        count: usize,
    },
    #[serde(rename = "assert.golden")]
    AssertGolden { actual: String, expected: String },
    #[serde(rename = "assert.golden_missing_xfail")]
    AssertGoldenMissingXfail {
        actual: String,
        expected: String,
        reason: String,
    },
    #[serde(rename = "assert.golden_any")]
    AssertGoldenAny {
        actual: String,
        expected: Vec<String>,
    },
    #[serde(rename = "assert.golden_native")]
    AssertGoldenNative { actual: String, expected: String },
    #[serde(rename = "assert.golden_normalized")]
    AssertGoldenNormalized {
        actual: String,
        expected: String,
        normalizations: Vec<GoldenNormalization>,
    },
    #[serde(rename = "assert.golden_sorted_lines")]
    AssertGoldenSortedLines { actual: String, expected: String },
    #[serde(rename = "assert.golden_xfail")]
    AssertGoldenXfail {
        actual: String,
        expected: String,
        reason: String,
    },
    #[serde(rename = "assert.verilog")]
    AssertVerilog { actual: String, expected: String },
    #[serde(rename = "assert.vcd")]
    AssertVcd { actual: String, expected: String },
    #[serde(rename = "assert.vcd_valid")]
    AssertVcdValid { path: String },
    #[serde(rename = "assert.vcd_valid_if_present")]
    AssertVcdValidIfPresent { path: String },
}

impl Action {
    pub fn asserted_path(&self) -> Option<&str> {
        match self {
            Self::AssertExists { path }
            | Self::AssertTextContains { path, .. }
            | Self::AssertTextAbsent { path, .. }
            | Self::AssertRegex { path, .. }
            | Self::AssertRegexAbsent { path, .. }
            | Self::AssertTextCount { path, .. }
            | Self::AssertRegexCount { path, .. }
            | Self::AssertDiagnosticCount { path, .. }
            | Self::VcdCheck { path, .. }
            | Self::AssertVcdValid { path }
            | Self::AssertVcdValidIfPresent { path } => Some(path),
            Self::AssertGolden { actual, .. }
            | Self::AssertGoldenMissingXfail { actual, .. }
            | Self::AssertGoldenAny { actual, .. }
            | Self::AssertGoldenNative { actual, .. }
            | Self::AssertGoldenNormalized { actual, .. }
            | Self::AssertGoldenSortedLines { actual, .. }
            | Self::AssertGoldenXfail { actual, .. }
            | Self::AssertVerilog { actual, .. }
            | Self::AssertVcd { actual, .. } => Some(actual),
            Self::BscCompile { .. }
            | Self::BscOptions { .. }
            | Self::BscFlagPreflight { .. }
            | Self::BluetclRun { .. }
            | Self::MakeTestData
            | Self::InterraOperatorVectors { .. }
            | Self::Bsc2Bsv { .. }
            | Self::BscParsePretty { .. }
            | Self::DumpIntermediate { .. }
            | Self::RenderGolden { .. }
            | Self::M4CurdirRender { .. }
            | Self::TextNormalize { .. }
            | Self::VerilogFilter { .. }
            | Self::BscGenerate { .. }
            | Self::BscSimirExport { .. }
            | Self::SimirM0Step { .. }
            | Self::SimirM2Run { .. }
            | Self::SimirM3Run { .. }
            | Self::CObjectBuild { .. }
            | Self::BscLink { .. }
            | Self::SimulationRun { .. }
            | Self::ShowRules { .. }
            | Self::BscSystemcLink { .. }
            | Self::SystemcCxxLink { .. }
            | Self::SystemcRun { .. }
            | Self::FsCopy { .. }
            | Self::FsCopyReplace { .. }
            | Self::FsRewriteDarwinCppIncludePath { .. }
            | Self::FsMove { .. }
            | Self::FsMoveReplace { .. }
            | Self::FsRemove { .. }
            | Self::FsEnsureAbsent { .. }
            | Self::FsEnsureDirectoryAbsent { .. }
            | Self::FsMkdir { .. }
            | Self::FsCreateDirAll { .. }
            | Self::FsTouch { .. }
            | Self::FsTouchCreate { .. }
            | Self::FsRemoveUserRead { .. }
            | Self::Delay { .. } => None,
        }
    }

    pub fn expected_paths(&self) -> Vec<&str> {
        match self {
            Self::AssertGolden { expected, .. }
            | Self::AssertGoldenNative { expected, .. }
            | Self::AssertGoldenNormalized { expected, .. }
            | Self::AssertGoldenSortedLines { expected, .. }
            | Self::AssertGoldenXfail { expected, .. }
            | Self::AssertVerilog { expected, .. }
            | Self::AssertVcd { expected, .. } => vec![expected],
            Self::AssertGoldenAny { expected, .. } => expected.iter().map(String::as_str).collect(),
            _ => Vec::new(),
        }
    }

    pub fn is_assertion(&self) -> bool {
        self.asserted_path().is_some()
    }

    pub fn requires_non_windows(&self) -> bool {
        matches!(
            self,
            Self::BscLink {
                simulator: IcarusSimulatorSelector::PosixEchoProbe,
                ..
            }
        ) || action_paths(self)
            .into_iter()
            .any(path_requires_non_windows)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum BluetclInvocation {
    Script {
        script: String,
        args: Vec<String>,
        syntax: BluetclSyntax,
    },
    Exec {
        script: String,
        args: Vec<String>,
    },
    InstalledScript {
        script: BluetclInstalledScript,
        args: Vec<String>,
    },
    Makedepend {
        command: BluetclMakedependCommand,
        args: Vec<String>,
    },
}

impl BluetclInvocation {
    pub fn fixture_script(&self) -> Option<&str> {
        match self {
            Self::Script { script, .. } | Self::Exec { script, .. } => Some(script),
            Self::InstalledScript { .. } | Self::Makedepend { .. } => None,
        }
    }

    pub fn args(&self) -> &[String] {
        match self {
            Self::Script { args, .. }
            | Self::Exec { args, .. }
            | Self::InstalledScript { args, .. }
            | Self::Makedepend { args, .. } => args,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BluetclSyntax {
    Bsv,
    Bh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BluetclMakedependCommand {
    Makedepend,
    MakedependTcl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IntermediateDumpView {
    Bi,
    Bo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SimulationBackend {
    Bluesim,
    Icarus,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BscLinkMode {
    #[default]
    Standard,
    NoMain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BscFlagPreflightMode {
    VerilogNoOptUndetermined,
    BluesimLink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum UndeterminedValue {
    X,
    Z,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IcarusSimulatorSelector {
    #[default]
    Default,
    BluespecDirInstalledBuilder,
    PosixEchoProbe,
    LiteralBogus,
    BluespecDirBogus,
}

impl IcarusSimulatorSelector {
    fn is_default(&self) -> bool {
        *self == Self::Default
    }

    pub fn produces_executable(self) -> bool {
        matches!(self, Self::Default | Self::BluespecDirInstalledBuilder)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TextNormalization {
    SortNumericField1ThenField2,
    VerilogTaskProjection,
    BluesimTaskProjection,
    IfNestedToSplitIfNested,
    IfNestedToNoSplitIfNested,
    MakeDirectoryMessages,
    IverilogQuietOutput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VerilogFilterProfile {
    RenameFire,
    ClockToClock,
    #[serde(rename = "wf_to_w_f")]
    WfToWF,
    MissingSed,
}

impl VerilogFilterProfile {
    pub fn fixture_path(self) -> Option<&'static str> {
        match self {
            Self::RenameFire => Some("renamefire.pl"),
            Self::ClockToClock => Some("simple.sed"),
            Self::WfToWF => Some("order.sed"),
            Self::MissingSed => None,
        }
    }
}

impl BscLinkMode {
    fn is_standard(&self) -> bool {
        *self == Self::Standard
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SimulationGenerationMode {
    Bluesim,
    Verilog,
    SharedElaboration,
}

impl SimulationGenerationMode {
    pub fn compiler_output_path(self, source: &str) -> String {
        let phase = match self {
            Self::Bluesim => "ccomp",
            Self::Verilog | Self::SharedElaboration => "vcomp",
        };
        format!("{source}.bsc-{phase}-out")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GoldenReplacement {
    BluespecDir,
    WorkDir,
    FifoWarningLocations,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BscCompileEnvironment {
    #[serde(rename = "ghcrts_m1_2g")]
    GhcrtsM1_2g,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BscCompileMode {
    Frontend,
    BluesimObject,
    Verilog,
    VerilogSchedule,
    Synthesize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DependencyMode {
    Update,
    NoDeps,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InterraOperatorSuite {
    Arith,
    BitSel,
    Logic,
}

impl InterraOperatorSuite {
    pub fn generated_verilog(self) -> &'static str {
        match self {
            Self::Arith => "gen_arith.v",
            Self::BitSel => "gen_bits.v",
            Self::Logic => "gen_logic.v",
        }
    }

    pub fn verilog_top(self) -> &'static str {
        match self {
            Self::Arith => "gen_arith",
            Self::BitSel => "gen_bits",
            Self::Logic => "gen_logical",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExpectedExit {
    #[default]
    Success,
    Failure,
    Unchecked,
}

impl ExpectedExit {
    fn is_success(&self) -> bool {
        *self == Self::Success
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExpectedExitSet {
    pub codes: Vec<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aarch64_codes: Option<Vec<i32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub windows_codes: Option<Vec<i32>>,
}

impl ExpectedExitSet {
    pub fn new(
        codes: Vec<i32>,
        aarch64_codes: Option<Vec<i32>>,
        windows_codes: Option<Vec<i32>>,
    ) -> Self {
        Self {
            codes: if codes.is_empty() { vec![0] } else { codes },
            aarch64_codes,
            windows_codes,
        }
    }

    pub fn is_success(&self) -> bool {
        self.codes == [0] && self.aarch64_codes.is_none() && self.windows_codes.is_none()
    }

    pub fn accepts_for_platform(&self, code: i32, aarch64: bool, windows: bool) -> bool {
        let codes = if windows {
            self.windows_codes.as_ref().unwrap_or(&self.codes)
        } else if aarch64 {
            self.aarch64_codes.as_ref().unwrap_or(&self.codes)
        } else {
            &self.codes
        };
        codes.contains(&code)
    }

    pub fn accepts_current_platform(&self, code: i32) -> bool {
        self.accepts_for_platform(
            code,
            cfg!(target_arch = "aarch64"),
            cfg!(target_os = "windows"),
        )
    }

    fn validate(&self) -> Result<(), ValidationError> {
        validate_exit_codes(&self.codes, "expected exit codes")?;
        if let Some(codes) = &self.aarch64_codes {
            validate_exit_codes(codes, "AArch64 expected exit codes")?;
        }
        if let Some(codes) = &self.windows_codes {
            validate_exit_codes(codes, "Windows expected exit codes")?;
        }
        Ok(())
    }
}

impl Default for ExpectedExitSet {
    fn default() -> Self {
        Self::new(vec![0], None, None)
    }
}

fn validate_exit_codes(codes: &[i32], label: &str) -> Result<(), ValidationError> {
    if codes.is_empty() {
        return Err(ValidationError::new(format!("{label} must not be empty")));
    }
    let unique = codes.iter().copied().collect::<BTreeSet<_>>();
    if unique.len() != codes.len() {
        return Err(ValidationError::new(format!(
            "{label} must not contain duplicates"
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticKind {
    Error,
    Warning,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum GoldenNormalization {
    GeneratedIds,
    SatSolverNames,
    VrWireIds,
    PreludePositions,
    PreludeBsvLineNumbers,
    CompilerBannerLines,
    WorkspaceRoot,
    LineDirectivePositions,
    BluetclOutput,
    BluetclPositionDigits,
    BluetclCregPositions,
    BluetclLibraries,
    BluetclPreludeLibrary,
    BracketedTimes,
    SplitIfRules,
    SystemVerilogTaskDiagnostics,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Provenance {
    pub span: SourceSpan,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceSpan {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportDiagnostic {
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub message: String,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TestPlanIndex {
    pub schema_version: u32,
    pub plans: Vec<TestPlanIndexEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TestPlanIndexEntry {
    pub id: String,
    pub path: String,
    pub origin: Origin,
    pub status: PlanStatus,
    pub scenario_count: usize,
    pub stage_count: usize,
    pub operation_count: usize,
    pub diagnostic_count: usize,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("invalid test plan: {message}")]
pub struct ValidationError {
    message: String,
}

impl ValidationError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl TestPlan {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.schema_version != TEST_PLAN_SCHEMA_VERSION {
            return Err(ValidationError::new(format!(
                "schema version {} is not supported",
                self.schema_version
            )));
        }
        if self.id.is_empty() || !is_safe_id(&self.id) {
            return Err(ValidationError::new("id must be a non-empty portable path"));
        }
        validate_path(&self.origin.path, "origin path")?;
        validate_hash(&self.origin.sha256, "origin hash")?;
        validate_path(&self.fixture_dir, "fixture directory")?;

        let mut fixture_paths = BTreeSet::new();
        let mut portable_fixture_paths = BTreeSet::new();
        for fixture in &self.fixtures {
            validate_path(&fixture.path, "fixture path")?;
            if let Some(source) = &fixture.source {
                validate_path(source, "fixture source")?;
                if source == &fixture.path {
                    return Err(ValidationError::new(format!(
                        "fixture {} aliases itself",
                        fixture.path
                    )));
                }
                if fixture.role != FixtureRole::Source {
                    return Err(ValidationError::new(format!(
                        "fixture alias {} must have the source role",
                        fixture.path
                    )));
                }
            }
            validate_hash(&fixture.sha256, "fixture hash")?;
            if !fixture_paths.insert(fixture.path.as_str())
                || !portable_fixture_paths.insert(fixture.path.to_ascii_lowercase())
            {
                return Err(ValidationError::new(format!(
                    "duplicate or case-colliding fixture {}",
                    fixture.path
                )));
            }
        }

        let mut scenario_ids = BTreeSet::new();
        for scenario in &self.scenarios {
            if scenario.id.is_empty() || !scenario_ids.insert(&scenario.id) {
                return Err(ValidationError::new(
                    "scenario ids must be non-empty and unique",
                ));
            }
            let mut scenario_fixture_paths = BTreeSet::new();
            let mut portable_scenario_fixture_paths = BTreeSet::new();
            for fixture in &scenario.fixtures {
                validate_path(fixture, "scenario fixture path")?;
                if !fixture_paths.contains(fixture.as_str()) {
                    return Err(ValidationError::new(format!(
                        "scenario {} references fixture {fixture} outside the plan fixture registry",
                        scenario.id
                    )));
                }
                if !scenario_fixture_paths.insert(fixture.as_str())
                    || !portable_scenario_fixture_paths.insert(fixture.to_ascii_lowercase())
                {
                    return Err(ValidationError::new(format!(
                        "scenario {} contains duplicate or case-colliding fixture {fixture}",
                        scenario.id
                    )));
                }
                if path_requires_non_windows(fixture)
                    && !scenario.requires.contains(&Requirement::NonWindows)
                {
                    return Err(ValidationError::new(format!(
                        "Windows-incompatible fixture {fixture} in scenario {} requires non_windows",
                        scenario.id
                    )));
                }
            }
            if scenario.bsc_options_append.as_ref().is_some_and(|append| {
                append.trim().is_empty()
                    || append.contains('\0')
                    || append.contains('\n')
                    || append.contains('\r')
            }) {
                return Err(ValidationError::new(format!(
                    "scenario {} has an empty or multiline BSC_OPTIONS append",
                    scenario.id
                )));
            }
            if scenario.requires.contains(&Requirement::PosixUnreadability) {
                return Err(ValidationError::new(format!(
                    "scenario {} must attach posix_unreadability only to dependent operations",
                    scenario.id
                )));
            }
            if scenario.requires.contains(&Requirement::Darwin) {
                return Err(ValidationError::new(format!(
                    "scenario {} must attach Darwin only to dependent operations",
                    scenario.id
                )));
            }
            if scenario
                .requires
                .iter()
                .any(|requirement| matches!(requirement, Requirement::BluetclPackage(_)))
            {
                return Err(ValidationError::new(format!(
                    "scenario {} must attach Bluetcl package availability only to guarded operations",
                    scenario.id
                )));
            }
            if scenario.requires.contains(&Requirement::Frontend)
                && scenario.requires.contains(&Requirement::Verilog)
            {
                return Err(ValidationError::new(format!(
                    "scenario {} cannot require both frontend and verilog profiles",
                    scenario.id
                )));
            }
            if scenario.stages.is_empty() {
                return Err(ValidationError::new(format!(
                    "scenario {} contains no stages",
                    scenario.id
                )));
            }
            if [
                scenario.timeouts.generation_seconds,
                scenario.timeouts.link_seconds,
                scenario.timeouts.simulation_seconds,
                scenario.timeouts.assertion_seconds,
            ]
            .contains(&0)
            {
                return Err(ValidationError::new(format!(
                    "scenario {} contains a zero timeout",
                    scenario.id
                )));
            }
            let mut stage_ids = BTreeSet::new();
            let mut produced_paths = BTreeSet::new();
            let mut portable_operation_paths = scenario
                .fixtures
                .iter()
                .map(|fixture| (fixture.to_ascii_lowercase(), fixture.clone()))
                .collect::<BTreeMap<_, _>>();
            for stage in &scenario.stages {
                if stage.id.is_empty() || !stage_ids.insert(&stage.id) {
                    return Err(ValidationError::new(format!(
                        "stage ids in scenario {} must be non-empty and unique",
                        scenario.id
                    )));
                }
                if stage.operations.is_empty() {
                    return Err(ValidationError::new(format!(
                        "stage {} contains no operations",
                        stage.id
                    )));
                }
                for operation in &stage.operations {
                    validate_operation(operation)?;
                    let paths = action_paths(&operation.action);
                    validate_artifact_contract(&operation.artifacts)?;
                    for path in paths
                        .iter()
                        .copied()
                        .chain(operation.artifacts.inputs.iter().map(String::as_str))
                        .chain(operation.artifacts.outputs.iter().map(String::as_str))
                        .chain(
                            operation
                                .artifacts
                                .output_alternatives
                                .iter()
                                .flatten()
                                .map(String::as_str),
                        )
                        .chain(operation.artifacts.directories.iter().map(String::as_str))
                        .chain(operation.artifacts.removes.iter().map(String::as_str))
                    {
                        let portable = path.to_ascii_lowercase();
                        if let Some(previous) = portable_operation_paths.get(&portable) {
                            if previous != path {
                                return Err(ValidationError::new(format!(
                                    "artifact paths {previous} and {path} collide on Windows"
                                )));
                            }
                        } else {
                            portable_operation_paths.insert(portable, path.to_owned());
                        }
                    }
                    if !scenario.requires.contains(&Requirement::NonWindows)
                        && paths.iter().any(|path| path_requires_non_windows(path))
                    {
                        return Err(ValidationError::new(format!(
                            "Windows-incompatible operation path in scenario {} requires non_windows",
                            scenario.id
                        )));
                    }
                    if self.status == PlanStatus::Complete {
                        for input in &operation.artifacts.inputs {
                            if !produced_paths.contains(input) {
                                if !fixture_paths.contains(input.as_str()) {
                                    return Err(ValidationError::new(format!(
                                        "operation input {input} in scenario {} for {:?} is neither produced nor registered as a fixture; preceding outputs: {produced_paths:?}",
                                        scenario.id, operation.action
                                    )));
                                }
                                if !scenario_fixture_paths.contains(input.as_str()) {
                                    return Err(ValidationError::new(format!(
                                        "fixture input {input} is not declared by scenario {}",
                                        scenario.id
                                    )));
                                }
                            }
                        }
                    }
                    if let Action::BscCompile { mode, .. } = &operation.action {
                        let required = match mode {
                            BscCompileMode::Frontend => None,
                            BscCompileMode::BluesimObject => Some(Requirement::Bluesim),
                            BscCompileMode::Verilog
                            | BscCompileMode::VerilogSchedule
                            | BscCompileMode::Synthesize => Some(Requirement::Verilog),
                        };
                        if required.is_some_and(|required| !scenario.requires.contains(&required)) {
                            return Err(ValidationError::new(format!(
                                "bsc.compile mode {mode:?} requires the {required:?} capability"
                            )));
                        }
                    }
                    if let Action::BscGenerate { mode, .. } = &operation.action {
                        let required = match mode {
                            SimulationGenerationMode::Bluesim => &[Requirement::Bluesim][..],
                            SimulationGenerationMode::Verilog => &[Requirement::Verilog][..],
                            SimulationGenerationMode::SharedElaboration => {
                                &[Requirement::Bluesim, Requirement::Verilog][..]
                            }
                        };
                        validate_requirements(
                            &scenario.id,
                            "bsc.generate",
                            required,
                            &scenario.requires,
                        )?;
                    }
                    match &operation.action {
                        Action::BscLink { backend, .. } | Action::SimulationRun { backend, .. } => {
                            let (name, required) = match (&operation.action, backend) {
                                (Action::BscLink { .. }, SimulationBackend::Bluesim) => {
                                    ("bsc.link", &[Requirement::Bluesim][..])
                                }
                                (Action::BscLink { .. }, SimulationBackend::Icarus) => {
                                    ("bsc.link", &[Requirement::Verilog, Requirement::Icarus][..])
                                }
                                (Action::SimulationRun { .. }, SimulationBackend::Bluesim) => {
                                    ("simulation.run", &[Requirement::Bluesim][..])
                                }
                                (Action::SimulationRun { .. }, SimulationBackend::Icarus) => (
                                    "simulation.run",
                                    &[Requirement::Verilog, Requirement::Icarus][..],
                                ),
                                _ => unreachable!("matched link or simulation action"),
                            };
                            validate_requirements(
                                &scenario.id,
                                name,
                                required,
                                &scenario.requires,
                            )?;
                        }
                        Action::BluetclRun { .. } => {
                            validate_requirements(
                                &scenario.id,
                                "bluetcl.run",
                                &[Requirement::Bluetcl],
                                &scenario.requires,
                            )?;
                        }
                        Action::ShowRules { .. } => {
                            validate_requirements(
                                &scenario.id,
                                "vcd.showrules",
                                &[Requirement::ShowRules],
                                &scenario.requires,
                            )?;
                        }
                        Action::BscSystemcLink { .. } => {
                            validate_requirements(
                                &scenario.id,
                                "bsc.systemc_link",
                                &[Requirement::SystemC, Requirement::Bluesim],
                                &scenario.requires,
                            )?;
                        }
                        Action::SystemcCxxLink { .. } => {
                            validate_requirements(
                                &scenario.id,
                                "systemc.cxx_link",
                                &[Requirement::SystemC],
                                &scenario.requires,
                            )?;
                        }
                        Action::SystemcRun { .. } => {
                            validate_requirements(
                                &scenario.id,
                                "systemc.run",
                                &[Requirement::SystemC],
                                &scenario.requires,
                            )?;
                        }
                        _ => {}
                    }
                    produced_paths.extend(operation.artifacts.outputs.iter().cloned());
                    produced_paths.extend(
                        operation
                            .artifacts
                            .output_alternatives
                            .iter()
                            .flatten()
                            .cloned(),
                    );
                    for removed in &operation.artifacts.removes {
                        produced_paths.remove(removed);
                    }
                }
            }
        }

        let has_error = self
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error);
        match self.status {
            PlanStatus::Complete if has_error => {
                return Err(ValidationError::new(
                    "complete plan contains an import error diagnostic",
                ));
            }
            PlanStatus::Complete if self.scenarios.is_empty() => {
                return Err(ValidationError::new("complete plan contains no scenarios"));
            }
            PlanStatus::Disabled if has_error => {
                return Err(ValidationError::new(
                    "disabled plan contains an import error diagnostic",
                ));
            }
            PlanStatus::Disabled if !self.scenarios.is_empty() => {
                return Err(ValidationError::new(
                    "disabled plan contains executable scenarios",
                ));
            }
            PlanStatus::Disabled
                if !matches!(
                    self.diagnostics.as_slice(),
                    [ImportDiagnostic {
                        severity: DiagnosticSeverity::Warning,
                        code,
                        ..
                    }] if code == "import.disabled"
                ) =>
            {
                return Err(ValidationError::new(
                    "disabled plan must contain exactly one import.disabled warning",
                ));
            }
            PlanStatus::Blocked if !has_error => {
                return Err(ValidationError::new(
                    "blocked plan must explain itself with an error diagnostic",
                ));
            }
            PlanStatus::Complete | PlanStatus::Disabled | PlanStatus::Blocked => {}
        }
        Ok(())
    }
}

impl TestPlanIndex {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.schema_version != TEST_PLAN_INDEX_SCHEMA_VERSION {
            return Err(ValidationError::new(
                "unsupported plan index schema version",
            ));
        }
        let mut ids = BTreeSet::new();
        let mut paths = BTreeSet::new();
        let mut portable_ids = BTreeSet::new();
        let mut portable_paths = BTreeSet::new();
        for plan in &self.plans {
            if !ids.insert(&plan.id)
                || !paths.insert(&plan.path)
                || !portable_ids.insert(plan.id.to_ascii_lowercase())
                || !portable_paths.insert(plan.path.to_ascii_lowercase())
            {
                return Err(ValidationError::new(
                    "plan index contains duplicate or case-colliding ids or paths",
                ));
            }
            validate_path(&plan.path, "plan path")?;
            validate_path(&plan.origin.path, "plan origin")?;
            validate_hash(&plan.origin.sha256, "plan origin hash")?;
        }
        Ok(())
    }
}

pub fn render_plan(plan: &TestPlan) -> Result<String, serde_json::Error> {
    let mut rendered = serde_json::to_string_pretty(plan)?;
    rendered.push('\n');
    Ok(rendered)
}

pub fn render_index(index: &TestPlanIndex) -> Result<String, serde_json::Error> {
    let mut rendered = serde_json::to_string_pretty(index)?;
    rendered.push('\n');
    Ok(rendered)
}

pub fn render_schema() -> Result<String, serde_json::Error> {
    let mut rendered = serde_json::to_string_pretty(&schema_for!(TestPlan))?;
    rendered.push('\n');
    Ok(rendered)
}

fn validate_operation(operation: &OperationRecord) -> Result<(), ValidationError> {
    let required = ArtifactContract::for_action(&operation.action);
    for (name, declared, required) in [
        ("input", &operation.artifacts.inputs, required.inputs),
        ("output", &operation.artifacts.outputs, required.outputs),
        (
            "directory",
            &operation.artifacts.directories,
            required.directories,
        ),
        ("removal", &operation.artifacts.removes, required.removes),
    ] {
        for path in required {
            if !declared.contains(&path) {
                return Err(ValidationError::new(format!(
                    "operation artifact contract is missing required {name} {path}"
                )));
            }
        }
    }
    for required in required.output_alternatives {
        if !operation.artifacts.output_alternatives.contains(&required) {
            return Err(ValidationError::new(format!(
                "operation artifact contract is missing required output alternatives {required:?}"
            )));
        }
    }
    if let OperationExpectation::Xfail { reason } = &operation.expectation {
        if reason.trim().is_empty() {
            return Err(ValidationError::new(
                "xfail operation expectation requires a non-empty reason",
            ));
        }
        if !matches!(
            operation.action,
            Action::BscCompile { .. } | Action::BscParsePretty { .. } | Action::BscLink { .. }
        ) && !operation.action.is_assertion()
        {
            return Err(ValidationError::new(
                "xfail operation expectation is only supported for bsc.compile, bsc.parse_pretty_roundtrip, bsc.link, and assertion actions",
            ));
        }
    }
    if matches!(operation.action, Action::FsRemoveUserRead { .. })
        && !operation
            .requires
            .contains(&Requirement::PosixUnreadability)
    {
        return Err(ValidationError::new(
            "fs.remove_user_read requires the operation-level posix_unreadability capability",
        ));
    }
    if operation
        .requires
        .contains(&Requirement::PosixUnreadability)
        && !matches!(
            operation.action,
            Action::FsRemoveUserRead { .. }
                | Action::BscCompile { .. }
                | Action::BscLink { .. }
                | Action::AssertExists { .. }
                | Action::AssertTextContains { .. }
                | Action::AssertTextAbsent { .. }
                | Action::AssertRegex { .. }
                | Action::AssertRegexAbsent { .. }
                | Action::AssertTextCount { .. }
                | Action::AssertRegexCount { .. }
                | Action::AssertDiagnosticCount { .. }
                | Action::AssertGolden { .. }
                | Action::AssertGoldenAny { .. }
                | Action::AssertGoldenNative { .. }
                | Action::AssertGoldenNormalized { .. }
                | Action::AssertGoldenSortedLines { .. }
                | Action::AssertGoldenXfail { .. }
                | Action::AssertVerilog { .. }
        )
    {
        return Err(ValidationError::new(
            "posix_unreadability is limited to the permission change and its dependent compile/link/assertion operations",
        ));
    }
    if matches!(
        operation.action,
        Action::BluetclRun {
            invocation: BluetclInvocation::InstalledScript {
                script: BluetclInstalledScript::ExpandPorts,
                ..
            },
            ..
        }
    ) && !operation
        .requires
        .contains(&Requirement::BluetclPackage(BluetclPackage::ExpandPorts))
    {
        return Err(ValidationError::new(
            "expandPorts bluetcl.run requires the operation-level expand_ports package capability",
        ));
    }
    if matches!(
        operation.action,
        Action::FsRewriteDarwinCppIncludePath { .. }
    ) && !operation.requires.contains(&Requirement::Darwin)
    {
        return Err(ValidationError::new(
            "fs.rewrite_darwin_cpp_include_path requires the operation-level Darwin capability",
        ));
    }
    if operation.requires.contains(&Requirement::Darwin)
        && !matches!(
            operation.action,
            Action::FsRewriteDarwinCppIncludePath { .. } | Action::FsMoveReplace { .. }
        )
    {
        return Err(ValidationError::new(
            "Darwin is limited to the audited cpp rewrite and its move-replace",
        ));
    }
    if operation
        .requires
        .iter()
        .any(|requirement| matches!(requirement, Requirement::InternalChecks))
        && !matches!(
            operation.action,
            Action::Bsc2Bsv { .. }
                | Action::DumpIntermediate { .. }
                | Action::VcdCheck { .. }
                | Action::AssertExists { .. }
                | Action::AssertTextContains { .. }
                | Action::AssertTextAbsent { .. }
                | Action::AssertRegex { .. }
                | Action::AssertRegexAbsent { .. }
                | Action::AssertTextCount { .. }
                | Action::AssertRegexCount { .. }
                | Action::AssertGolden { .. }
                | Action::AssertGoldenAny { .. }
                | Action::AssertGoldenNative { .. }
                | Action::AssertGoldenNormalized { .. }
                | Action::AssertGoldenSortedLines { .. }
                | Action::AssertGoldenXfail { .. }
        )
    {
        return Err(ValidationError::new(
            "internal_checks operation requirement is limited to dump and assertion actions",
        ));
    }
    validate_action(&operation.action)
}

fn validate_action(action: &Action) -> Result<(), ValidationError> {
    let unchecked_exit = match action {
        Action::BluetclRun { expected_exit, .. }
        | Action::BscLink { expected_exit, .. }
        | Action::BscSystemcLink { expected_exit, .. }
        | Action::VcdCheck { expected_exit, .. } => {
            matches!(expected_exit, ExpectedExit::Unchecked)
        }
        _ => false,
    };
    if unchecked_exit {
        return Err(ValidationError::new(
            "unchecked exit status is limited to bsc.compile",
        ));
    }
    match action {
        Action::Delay { milliseconds } => {
            if !(1..=10_000).contains(milliseconds) {
                return Err(ValidationError::new(
                    "time.delay milliseconds must be between 1 and 10000",
                ));
            }
        }
        Action::FsTouchCreate {
            delay_milliseconds, ..
        } => {
            if !(1..=10_000).contains(delay_milliseconds) {
                return Err(ValidationError::new(
                    "fs.touch_create delayMilliseconds must be between 1 and 10000",
                ));
            }
        }
        Action::BscCompile {
            mode,
            module,
            args,
            absolute_import_paths,
            dependency_mode,
            expected_exit,
            unexpected_success_forbidden_regex,
            ..
        } => {
            validate_argv(args)?;
            if !absolute_import_paths.is_empty()
                && args
                    .iter()
                    .any(|arg| matches!(arg.as_str(), "-p" | "-vsearch"))
            {
                return Err(ValidationError::new(
                    "bsc.compile absolute import paths cannot be combined with -p or -vsearch",
                ));
            }
            let mut unique_imports = BTreeSet::new();
            for path in absolute_import_paths {
                validate_path(path, "bsc.compile absolute import path")?;
                if !unique_imports.insert(path) {
                    return Err(ValidationError::new(
                        "bsc.compile absolute import paths must be unique",
                    ));
                }
            }
            if let Some(module) = module {
                validate_portable_segment(module, "bsc.compile module")?;
            }
            if let Some(pattern) = unexpected_success_forbidden_regex {
                if !matches!(expected_exit, ExpectedExit::Failure) {
                    return Err(ValidationError::new(
                        "bsc.compile unexpected-success regex requires expectedExit failure",
                    ));
                }
                if pattern.is_empty() || Regex::new(pattern).is_err() {
                    return Err(ValidationError::new(
                        "bsc.compile unexpected-success regex must be non-empty and valid",
                    ));
                }
            }
            match (mode, module, dependency_mode) {
                (BscCompileMode::Frontend, Some(_), _) => {
                    return Err(ValidationError::new(
                        "frontend bsc.compile cannot select a module",
                    ));
                }
                (BscCompileMode::Frontend, None, _) => {}
                (BscCompileMode::Synthesize, _, DependencyMode::NoDeps) => {}
                (BscCompileMode::Synthesize, _, DependencyMode::Update) => {
                    return Err(ValidationError::new(
                        "synthesize bsc.compile must disable dependency updates",
                    ));
                }
                (_, _, DependencyMode::NoDeps) => {
                    return Err(ValidationError::new(
                        "non-synthesize backend bsc.compile cannot disable dependency updates",
                    ));
                }
                (_, _, DependencyMode::Update) => {}
            }
        }
        Action::BscOptions {
            args,
            expected_exit,
            bsc_options_prepend,
            ..
        } => {
            validate_bsc_options_argv(args)?;
            if matches!(expected_exit, ExpectedExit::Unchecked) {
                return Err(ValidationError::new(
                    "bsc.options does not support unchecked exit status",
                ));
            }
            if bsc_options_prepend.as_ref().is_some_and(|prepend| {
                prepend.trim().is_empty()
                    || prepend.contains('\0')
                    || prepend.contains(['\r', '\n'])
            }) {
                return Err(ValidationError::new(
                    "bsc.options BSC_OPTIONS prepend must be non-empty, single-line text",
                ));
            }
        }
        Action::BscFlagPreflight {
            mode, input, top, ..
        } => match mode {
            BscFlagPreflightMode::VerilogNoOptUndetermined => {
                if top.is_some()
                    || !matches!(
                        std::path::Path::new(input)
                            .extension()
                            .and_then(|ext| ext.to_str()),
                        Some("bs" | "bsv")
                    )
                {
                    return Err(ValidationError::new(
                        "Verilog no-opt-undetermined preflight requires a .bs/.bsv input and no top",
                    ));
                }
            }
            BscFlagPreflightMode::BluesimLink => {
                let Some(top) = top else {
                    return Err(ValidationError::new(
                        "Bluesim link flag preflight requires a top",
                    ));
                };
                validate_portable_segment(top, "bsc.flag_preflight top")?;
                if std::path::Path::new(input)
                    .extension()
                    .and_then(|extension| extension.to_str())
                    != Some("ba")
                {
                    return Err(ValidationError::new(
                        "Bluesim link flag preflight requires an explicit .ba input",
                    ));
                }
            }
        },
        Action::BluetclRun {
            invocation,
            working_directory,
            artifact_inputs,
            artifact_outputs,
            expected_exit,
            stdout,
        } => {
            if matches!(expected_exit, ExpectedExit::Unchecked) {
                return Err(ValidationError::new(
                    "bluetcl.run does not support unchecked exit status",
                ));
            }
            if let Some(script) = invocation.fixture_script() {
                if !script.ends_with(".tcl") {
                    return Err(ValidationError::new(
                        "bluetcl.run fixture script must be an explicit .tcl path",
                    ));
                }
            }
            match invocation {
                BluetclInvocation::InstalledScript { script, args } => {
                    validate_bluetcl_installed_script_invocation(
                        *script,
                        args,
                        working_directory.as_deref(),
                        artifact_inputs,
                        artifact_outputs,
                        *expected_exit,
                        stdout,
                    )?;
                }
                BluetclInvocation::Makedepend { command, args } => {
                    validate_bluetcl_makedepend_invocation(
                        *command,
                        args,
                        working_directory.as_deref(),
                        artifact_inputs,
                        artifact_outputs,
                        *expected_exit,
                        stdout,
                    )?;
                }
                _ => validate_argv(invocation.args())?,
            }
            for (label, paths) in [("input", artifact_inputs), ("output", artifact_outputs)] {
                let mut declared = BTreeSet::new();
                for path in paths {
                    if !declared.insert(path.to_ascii_lowercase()) {
                        return Err(ValidationError::new(format!(
                            "bluetcl.run artifact {label}s must be unique on Windows"
                        )));
                    }
                }
            }
        }
        Action::Bsc2Bsv { source, stdout } => {
            if !source.ends_with(".bs") || stdout != &format!("{source}.bsc2bsv-out") {
                return Err(ValidationError::new(
                    "internal.bsc2bsv requires a .bs source and its canonical .bsc2bsv-out path",
                ));
            }
        }
        Action::BscParsePretty {
            source,
            args,
            pretty_output,
        } => {
            validate_argv(args)?;
            let extension = if source.ends_with(".bsv") {
                "bsv"
            } else if source.ends_with(".bs") {
                "bs"
            } else {
                return Err(ValidationError::new(
                    "bsc.parse_pretty_roundtrip source must end in .bs or .bsv",
                ));
            };
            if pretty_output != &format!("{source}-pretty-out.{extension}") {
                return Err(ValidationError::new(
                    "bsc.parse_pretty_roundtrip requires its canonical pretty output path",
                ));
            }
        }
        Action::DumpIntermediate { input, output, .. } => {
            if !input.ends_with(".bo") || input == output {
                return Err(ValidationError::new(
                    "internal.dump input must be a distinct .bo artifact",
                ));
            }
        }
        Action::RenderGolden {
            template, output, ..
        }
        | Action::M4CurdirRender { template, output }
            if template.eq_ignore_ascii_case(output) =>
        {
            return Err(ValidationError::new(
                "golden.render template and output paths must not collide on Windows",
            ));
        }
        Action::RenderGolden { .. } | Action::M4CurdirRender { .. } => {}
        Action::TextNormalize {
            source,
            destination,
            ..
        } => {
            if source.eq_ignore_ascii_case(destination) {
                return Err(ValidationError::new(
                    "text.normalize source and destination must not collide on Windows",
                ));
            }
        }
        Action::VerilogFilter {
            profiles,
            expected_exit,
            ..
        } => {
            if profiles.is_empty() {
                return Err(ValidationError::new(
                    "verilog.filter requires at least one profile",
                ));
            }
            let missing = profiles
                .iter()
                .position(|profile| *profile == VerilogFilterProfile::MissingSed);
            let expected_failure = *expected_exit == ExpectedExit::Failure;
            if missing != expected_failure.then_some(profiles.len() - 1)
                || profiles[..profiles.len().saturating_sub(1)]
                    .contains(&VerilogFilterProfile::MissingSed)
            {
                return Err(ValidationError::new(
                    "verilog.filter missing_sed is legal only once, last, with expectedExit failure",
                ));
            }
        }
        Action::BscGenerate { module, args, .. } => {
            validate_argv(args)?;
            if let Some(module) = module {
                validate_portable_segment(module, "bsc.generate module")?;
            }
        }
        Action::BscSimirExport { top, output } => {
            validate_portable_segment(top, "bsc.simir_export top")?;
            if !output.ends_with(".bsim.json") {
                return Err(ValidationError::new(
                    "bsc.simir_export output must end with .bsim.json",
                ));
            }
        }
        Action::SimirM0Step { model, cycles, .. } => {
            if !model.ends_with(".bsim.json") {
                return Err(ValidationError::new(
                    "simir.m0_step model must end with .bsim.json",
                ));
            }
            if !(1..=1_000_000).contains(cycles) {
                return Err(ValidationError::new(
                    "simir.m0_step cycles must be between 1 and 1000000",
                ));
            }
        }
        Action::SimirM2Run {
            model, max_events, ..
        } => {
            if !model.ends_with(".m2.bsim.json") {
                return Err(ValidationError::new(
                    "simir.m2_run model must end with .m2.bsim.json",
                ));
            }
            if !(1..=1_000_000).contains(max_events) {
                return Err(ValidationError::new(
                    "simir.m2_run maxEvents must be between 1 and 1000000",
                ));
            }
        }
        Action::SimirM3Run {
            model, max_events, ..
        } => {
            if !model.ends_with(".m3.bsim.json") {
                return Err(ValidationError::new(
                    "simir.m3_run model must end with .m3.bsim.json",
                ));
            }
            if !(1..=1_000_000).contains(max_events) {
                return Err(ValidationError::new(
                    "simir.m3_run maxEvents must be between 1 and 1000000",
                ));
            }
        }
        Action::CObjectBuild {
            source,
            makefile,
            output,
        } => {
            let source_path = std::path::Path::new(source);
            let expected_output_path = source_path.with_extension("o");
            let expected_output = expected_output_path.to_string_lossy();
            if source_path.extension().and_then(|value| value.to_str()) != Some("c")
                || std::path::Path::new(makefile)
                    .extension()
                    .and_then(|value| value.to_str())
                    != Some("mk")
                || output != expected_output.as_ref()
            {
                return Err(ValidationError::new(
                    "c.compile_object requires a .c source, a .mk makefile, and the source's canonical .o output",
                ));
            }
        }
        Action::SimulationRun {
            args,
            expected_exits,
            ..
        } => {
            validate_argv(args)?;
            validate_simulation_file_options(args)?;
            expected_exits.validate()?;
        }
        Action::BscLink {
            backend,
            mode,
            objects,
            top,
            args,
            expected_exit,
            missing_objects,
            simulator,
        } => {
            validate_argv(objects)?;
            validate_argv(args)?;
            validate_portable_segment(top, "bsc.link top")?;
            if objects
                .iter()
                .filter(|object| missing_objects.contains(object))
                .count()
                != missing_objects.len()
            {
                return Err(ValidationError::new(
                    "bsc.link missing objects must be a subset of the link objects",
                ));
            }
            if *mode == BscLinkMode::NoMain
                && (*backend != SimulationBackend::Icarus
                    || *expected_exit != ExpectedExit::Success
                    || *simulator != IcarusSimulatorSelector::Default)
            {
                return Err(ValidationError::new(
                    "bsc.link no_main mode requires the Icarus backend, default simulator, and a successful exit",
                ));
            }
            if *simulator != IcarusSimulatorSelector::Default
                && (*backend != SimulationBackend::Icarus || *mode != BscLinkMode::Standard)
            {
                return Err(ValidationError::new(
                    "bsc.link non-default simulator selectors require standard Icarus linking",
                ));
            }

            if !simulator.produces_executable()
                && *expected_exit == ExpectedExit::Success
                && !matches!(simulator, IcarusSimulatorSelector::PosixEchoProbe)
            {
                return Err(ValidationError::new(
                    "bsc.link successful non-producing selector is limited to posix_echo_probe",
                ));
            }
        }
        Action::BscSystemcLink { objects, top, .. } => {
            if objects.is_empty() {
                return Err(ValidationError::new(
                    "bsc.systemc_link requires at least one object",
                ));
            }
            for object in objects {
                if !object.ends_with(".ba") {
                    return Err(ValidationError::new(
                        "bsc.systemc_link objects must be explicit .ba paths",
                    ));
                }
            }
            validate_portable_segment(top, "bsc.systemc_link top")?;
        }
        Action::SystemcCxxLink {
            executable,
            sources,
            top_modules,
            other_modules,
            defines,
        } => {
            if sources.is_empty() {
                return Err(ValidationError::new(
                    "systemc.cxx_link requires at least one source",
                ));
            }
            if top_modules.is_empty() {
                return Err(ValidationError::new(
                    "systemc.cxx_link requires at least one top module",
                ));
            }
            validate_portable_segment(executable, "systemc.cxx_link executable")?;
            for module in top_modules.iter().chain(other_modules) {
                validate_portable_segment(module, "systemc.cxx_link module")?;
            }
            for define in defines {
                if !is_safe_systemc_define(define) {
                    return Err(ValidationError::new(
                        "systemc.cxx_link defines must start with -D and contain no whitespace or path characters",
                    ));
                }
            }
        }
        Action::SystemcRun { executable, .. } => {
            validate_portable_segment(executable, "systemc.run executable")?;
        }
        Action::ShowRules {
            top,
            input,
            output,
            design_inputs,
            stdout,
        } => {
            validate_portable_segment(top, "vcd.showrules top")?;
            if !input.ends_with(".vcd")
                || !output.ends_with(".vcd")
                || input.eq_ignore_ascii_case(output)
            {
                return Err(ValidationError::new(
                    "vcd.showrules requires distinct .vcd input and output paths",
                ));
            }
            if stdout != &format!("{input}.showrules-out") {
                return Err(ValidationError::new(
                    "vcd.showrules requires the canonical input.vcd.showrules-out stdout path",
                ));
            }
            if design_inputs.is_empty() {
                return Err(ValidationError::new(
                    "vcd.showrules requires a proven non-empty .ba design input hierarchy",
                ));
            }
            let mut portable = BTreeSet::new();
            for design_input in design_inputs {
                if !design_input.ends_with(".ba") {
                    return Err(ValidationError::new(
                        "vcd.showrules design inputs must be explicit .ba paths",
                    ));
                }
                if !portable.insert(design_input.to_ascii_lowercase()) {
                    return Err(ValidationError::new(
                        "vcd.showrules design inputs must be unique on Windows",
                    ));
                }
            }
            if !design_inputs
                .iter()
                .any(|path| path == &format!("{top}.ba"))
            {
                return Err(ValidationError::new(
                    "vcd.showrules design inputs must include the top .ba artifact",
                ));
            }
        }
        Action::VcdCheck { checks, .. } => {
            if checks.is_empty() {
                return Err(ValidationError::new(
                    "vcd.check requires at least one check",
                ));
            }
            validate_argv(checks)?;
        }
        Action::AssertGoldenNormalized { normalizations, .. }
            if normalizations.is_empty()
                || normalizations.iter().collect::<BTreeSet<_>>().len() != normalizations.len() =>
        {
            return Err(ValidationError::new(
                "assert.golden_normalized requires unique normalization profiles",
            ));
        }
        Action::AssertGoldenAny { actual, expected } => {
            if expected.is_empty() {
                return Err(ValidationError::new(
                    "assert.golden_any requires at least one expected path",
                ));
            }
            let mut paths = BTreeSet::new();
            for path in expected {
                if path.eq_ignore_ascii_case(actual) {
                    return Err(ValidationError::new(
                        "comparison actual and expected paths must not collide on Windows",
                    ));
                }
                if !paths.insert(path.to_ascii_lowercase()) {
                    return Err(ValidationError::new(
                        "assert.golden_any expected paths must be unique on Windows",
                    ));
                }
            }
        }
        _ => {}
    }
    let paths = action_paths(action);
    if let Action::AssertGolden { actual, expected }
    | Action::AssertGoldenNative { actual, expected }
    | Action::AssertGoldenNormalized {
        actual, expected, ..
    }
    | Action::AssertGoldenSortedLines { actual, expected }
    | Action::AssertVerilog { actual, expected }
    | Action::AssertVcd { actual, expected } = action
    {
        if actual.eq_ignore_ascii_case(expected) {
            return Err(ValidationError::new(
                "comparison actual and expected paths must not collide on Windows",
            ));
        }
    }
    for path in paths {
        validate_path(path, "operation path")?;
    }
    Ok(())
}

fn validate_artifact_contract(artifacts: &ArtifactContract) -> Result<(), ValidationError> {
    for paths in [
        &artifacts.inputs,
        &artifacts.outputs,
        &artifacts.directories,
        &artifacts.removes,
    ] {
        let mut declared = BTreeSet::new();
        for path in paths {
            validate_path(path, "artifact contract path")?;
            if !declared.insert(path.to_ascii_lowercase()) {
                return Err(ValidationError::new(format!(
                    "artifact contract contains duplicate or case-colliding path {path}"
                )));
            }
        }
    }
    for alternatives in &artifacts.output_alternatives {
        if alternatives.is_empty() {
            return Err(ValidationError::new(
                "artifact output alternatives must not be empty",
            ));
        }
        let mut declared = BTreeSet::new();
        for path in alternatives {
            validate_path(path, "artifact output alternative")?;
            if !declared.insert(path.to_ascii_lowercase()) {
                return Err(ValidationError::new(format!(
                    "artifact output alternatives contain duplicate or case-colliding path {path}"
                )));
            }
        }
    }
    Ok(())
}

fn validate_bluetcl_installed_script_invocation(
    script: BluetclInstalledScript,
    args: &[String],
    working_directory: Option<&str>,
    artifact_inputs: &[String],
    artifact_outputs: &[String],
    expected_exit: ExpectedExit,
    stdout: &str,
) -> Result<(), ValidationError> {
    const EXPAND_PORTS_CASES: [&str; 13] = [
        "Test1", "Test10", "Test1a", "Test1b", "Test2", "Test3", "Test4", "Test5", "Test6",
        "Test7", "Test7a", "Test7b", "Test12",
    ];
    let valid = match script {
        BluetclInstalledScript::ExpandPorts => EXPAND_PORTS_CASES.iter().any(|package| {
            let module = format!("mk{package}");
            let wrapper = format!("{module}.wrapper.got.v");
            let include = format!("{module}.includes.got.vh");
            let mut expected_args = vec!["-quiet".to_owned()];
            let mut expected_inputs = vec![
                format!("{package}.bo"),
                format!("{module}.ba"),
                format!("{module}.v"),
            ];
            if *package == "Test7b" {
                let rename = format!("{package}.rename.tcl");
                expected_args.extend(["-rename".to_owned(), rename.clone()]);
                expected_inputs.push(rename);
            }
            expected_args.extend([
                "-wrapper".to_owned(),
                wrapper.clone(),
                "-include".to_owned(),
                include.clone(),
                (*package).to_owned(),
                module.clone(),
                format!("{module}.v"),
            ]);
            working_directory.is_none()
                && expected_exit == ExpectedExit::Success
                && args == expected_args
                && artifact_inputs == expected_inputs
                && artifact_outputs == [wrapper, include]
                && stdout == format!("{package}.expandPorts.bluetcl-out")
        }),
    };
    valid.then_some(()).ok_or_else(|| {
        ValidationError::new(
            "bluetcl.run installed script invocation is outside the audited static contract",
        )
    })
}

fn validate_bluetcl_makedepend_invocation(
    command: BluetclMakedependCommand,
    args: &[String],
    working_directory: Option<&str>,
    artifact_inputs: &[String],
    artifact_outputs: &[String],
    expected_exit: ExpectedExit,
    stdout: &str,
) -> Result<(), ValidationError> {
    const INPUTS: [&str; 7] = [
        "Dep1.bsv",
        "Foo.bsv",
        "IncDep1.bsv",
        "IncDep2.bsv",
        "Test.bsv",
        "include1.inc",
        "subinclude.inc",
    ];
    let words = args.iter().map(String::as_str).collect::<Vec<_>>();
    let contract = match command {
        BluetclMakedependCommand::MakedependTcl if words.is_empty() => {
            Some(("usage2", ExpectedExit::Failure, None))
        }
        BluetclMakedependCommand::MakedependTcl => None,
        BluetclMakedependCommand::Makedepend => match words.as_slice() {
            [] => Some(("usage1", ExpectedExit::Failure, None)),
            ["-v"] => Some(("nofile", ExpectedExit::Failure, None)),
            ["-xxx", "Dep1.bsv"] => Some(("badflag", ExpectedExit::Failure, None)),
            ["-D", "SYNTAXERROR", "Dep1.bsv"] => Some(("error1", ExpectedExit::Failure, None)),
            ["-D", "CIRCERROR", "Dep1.bsv"] => Some(("error2", ExpectedExit::Failure, None)),
            ["-no-show-timestamps", "Dep1.bsv"] => Some(("test1", ExpectedExit::Success, None)),
            ["-no-show-timestamps", "*.bsv"] => Some(("patterns", ExpectedExit::Success, None)),
            ["-no-show-timestamps", "-D", "INC1", "Dep1.bsv"] => {
                Some(("defines", ExpectedExit::Success, None))
            }
            ["-no-show-timestamps", "-bdir", "objs", "-D", "INC1", "Dep1.bsv"] => {
                Some(("bdir", ExpectedExit::Success, None))
            }
            ["-no-show-timestamps", "-bdir", "objs", "-p", "../makedepend/:%/Libraries", "-D", "INC1", "Dep1.bsv"] => {
                Some(("updir", ExpectedExit::Success, Some("makedepend")))
            }
            ["-no-show-timestamps", "-o", "minusO.depend-out", "Dep1.bsv"] => {
                Some(("minus_o", ExpectedExit::Success, None))
            }
            _ => None,
        },
    };
    let Some((output_name, contract_exit, contract_directory)) = contract else {
        return Err(ValidationError::new(
            "bluetcl.run makedepend invocation is outside the audited static command set",
        ));
    };
    let needs_inputs = !matches!(output_name, "usage1" | "usage2" | "nofile");
    let expected_inputs = needs_inputs
        .then(|| {
            INPUTS
                .iter()
                .map(|path| {
                    contract_directory.map_or_else(
                        || (*path).to_owned(),
                        |directory| format!("{directory}/{path}"),
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let expected_outputs = (output_name == "minus_o")
        .then(|| vec!["minusO.depend-out".to_owned()])
        .unwrap_or_default();
    let expected_stdout = contract_directory.map_or_else(
        || format!("{output_name}.bluetcl-out"),
        |directory| format!("{directory}/{output_name}.bluetcl-out"),
    );
    (working_directory == contract_directory
        && expected_exit == contract_exit
        && artifact_inputs == expected_inputs
        && artifact_outputs == expected_outputs
        && stdout == expected_stdout)
        .then_some(())
        .ok_or_else(|| {
            ValidationError::new(
                "bluetcl.run makedepend invocation does not match its audited exit/artifact contract",
            )
        })
}

fn action_paths(action: &Action) -> Vec<&str> {
    match action {
        Action::BscCompile {
            source,
            working_directory,
            stdout,
            ..
        } => {
            let mut paths = vec![source.as_str(), stdout.as_str()];
            paths.extend(working_directory.iter().map(String::as_str));
            paths
        }
        Action::BscOptions { stdout, .. } => vec![stdout],
        Action::BscFlagPreflight {
            input, top, stdout, ..
        } => {
            let mut paths = vec![input.as_str(), stdout.as_str()];
            paths.extend(top.iter().map(String::as_str));
            paths
        }
        Action::BluetclRun {
            invocation,
            working_directory,
            artifact_inputs,
            artifact_outputs,
            stdout,
            ..
        } => {
            let mut paths = Vec::new();
            paths.extend(invocation.fixture_script());
            paths.extend(working_directory.iter().map(String::as_str));
            paths.extend(artifact_inputs.iter().map(String::as_str));
            paths.extend(artifact_outputs.iter().map(String::as_str));
            paths.push(stdout.as_str());
            paths
        }
        Action::MakeTestData | Action::InterraOperatorVectors { .. } | Action::Delay { .. } => {
            Vec::new()
        }
        Action::Bsc2Bsv { source, stdout } => vec![source, stdout],
        Action::BscParsePretty {
            source,
            pretty_output,
            ..
        } => vec![source, pretty_output],
        Action::DumpIntermediate { input, output, .. } => vec![input, output],
        Action::RenderGolden {
            template, output, ..
        }
        | Action::M4CurdirRender { template, output } => vec![template, output],
        Action::TextNormalize {
            source,
            destination,
            ..
        } => vec![source, destination],
        Action::VerilogFilter { path, profiles, .. } => {
            let mut paths = vec![path.as_str()];
            paths.extend(profiles.iter().filter_map(|profile| profile.fixture_path()));
            paths
        }
        Action::BscGenerate { source, .. } => vec![source],
        Action::BscSimirExport { top, output } => vec![top, output],
        Action::SimirM0Step { model, stdout, .. }
        | Action::SimirM2Run { model, stdout, .. }
        | Action::SimirM3Run { model, stdout, .. } => vec![model, stdout],
        Action::CObjectBuild {
            source,
            makefile,
            output,
        } => vec![source, makefile, output],
        Action::BscLink { objects, args, .. } => {
            let mut paths = objects.iter().map(String::as_str).collect::<Vec<_>>();
            paths.extend(
                args.windows(2)
                    .filter(|pair| pair[0] == "-Xv")
                    .map(|pair| pair[1].as_str()),
            );
            paths
        }
        Action::BscSystemcLink { objects, .. } => objects.iter().map(String::as_str).collect(),
        Action::SystemcCxxLink { sources, .. } => sources.iter().map(String::as_str).collect(),
        Action::SystemcRun {
            executable, stdout, ..
        } => vec![executable, stdout],
        Action::SimulationRun {
            executable,
            args,
            stdout,
            vcd,
            ..
        } => {
            let mut paths = vec![executable.as_str(), stdout.as_str()];
            paths.extend(simulation_file_inputs(args));
            if let Some(vcd) = vcd {
                paths.push(vcd);
            }
            paths
        }
        Action::ShowRules {
            top: _,
            input,
            output,
            design_inputs,
            stdout,
        } => {
            let mut paths = vec![input.as_str(), output.as_str(), stdout.as_str()];
            paths.extend(design_inputs.iter().map(String::as_str));
            paths
        }
        Action::VcdCheck { path, .. } => vec![path],
        Action::FsCopy {
            source,
            destination,
        }
        | Action::FsCopyReplace {
            source,
            destination,
        }
        | Action::FsRewriteDarwinCppIncludePath {
            source,
            destination,
        }
        | Action::FsMove {
            source,
            destination,
        }
        | Action::FsMoveReplace {
            source,
            destination,
        } => vec![source, destination],
        Action::FsRemove { path }
        | Action::FsEnsureAbsent { path }
        | Action::FsEnsureDirectoryAbsent { path }
        | Action::FsMkdir { path }
        | Action::FsCreateDirAll { path }
        | Action::FsTouch { path }
        | Action::FsTouchCreate { path, .. }
        | Action::FsRemoveUserRead { path }
        | Action::AssertExists { path }
        | Action::AssertTextContains { path, .. }
        | Action::AssertTextAbsent { path, .. }
        | Action::AssertRegex { path, .. }
        | Action::AssertRegexAbsent { path, .. }
        | Action::AssertTextCount { path, .. }
        | Action::AssertRegexCount { path, .. }
        | Action::AssertDiagnosticCount { path, .. }
        | Action::AssertVcdValid { path }
        | Action::AssertVcdValidIfPresent { path } => vec![path],
        Action::AssertGolden { actual, expected }
        | Action::AssertGoldenMissingXfail {
            actual, expected, ..
        }
        | Action::AssertGoldenNative { actual, expected }
        | Action::AssertGoldenNormalized {
            actual, expected, ..
        }
        | Action::AssertGoldenSortedLines { actual, expected }
        | Action::AssertGoldenXfail {
            actual, expected, ..
        }
        | Action::AssertVerilog { actual, expected }
        | Action::AssertVcd { actual, expected } => vec![actual, expected],
        Action::AssertGoldenAny { actual, expected } => {
            let mut paths = vec![actual.as_str()];
            paths.extend(expected.iter().map(String::as_str));
            paths
        }
    }
}

fn validate_requirements(
    scenario: &str,
    operation: &str,
    required: &[Requirement],
    actual: &[Requirement],
) -> Result<(), ValidationError> {
    for requirement in required {
        if !actual.contains(requirement) {
            return Err(ValidationError::new(format!(
                "{operation} in scenario {scenario} requires the {requirement:?} capability"
            )));
        }
    }
    Ok(())
}

pub fn generation_static_dump_artifacts(args: &[String], module: Option<&str>) -> Vec<String> {
    args.iter()
        .filter_map(|argument| {
            let (option, path) = argument.split_once('=')?;
            match option {
                "-dATS" | "-dATSexpand" | "-dastate" | "-dsplitIf" => {}
                _ => return None,
            }
            if path.contains('%') {
                let module = module?;
                if path.matches("%m").count() != 1 || path.replace("%m", "").contains('%') {
                    return None;
                }
                Some(path.replace("%m", module))
            } else {
                Some(path.to_owned())
            }
        })
        .collect()
}

pub fn generation_package_artifacts(source: &str, args: &[String]) -> Vec<String> {
    if args.iter().any(|argument| argument.starts_with("-KILL")) {
        return Vec::new();
    }
    let package = std::path::Path::new(source)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(source);
    let directory = args
        .windows(2)
        .find_map(|window| (window[0] == "-bdir").then_some(window[1].trim_end_matches('/')))
        .filter(|directory| !directory.is_empty())
        .map_or_else(
            || {
                std::path::Path::new(source)
                    .parent()
                    .filter(|directory| !directory.as_os_str().is_empty())
                    .map_or_else(String::new, |directory| {
                        directory.to_string_lossy().into_owned()
                    })
            },
            str::to_owned,
        );
    let path = |extension| {
        if directory.is_empty() {
            format!("{package}.{extension}")
        } else {
            format!("{directory}/{package}.{extension}")
        }
    };
    vec![path("bo")]
}

pub fn simulation_executable_artifact(backend: SimulationBackend, executable: &str) -> String {
    match backend {
        SimulationBackend::Bluesim => format!("{executable}.cexe"),
        SimulationBackend::Icarus => format!("{executable}.vexe"),
    }
}

pub fn simulation_vcd_outputs(arguments: &[String]) -> Vec<String> {
    let mut outputs = Vec::new();
    let mut index = 0;
    while index < arguments.len() {
        if arguments[index] != "-V" {
            index += 1;
            continue;
        }
        let explicit = arguments
            .get(index + 1)
            .filter(|value| !value.starts_with(['-', '+']));
        outputs.push(explicit.map_or_else(|| "dump.vcd".to_owned(), Clone::clone));
        index += usize::from(explicit.is_some()) + 1;
    }
    outputs
}

fn link_file_inputs(arguments: &[String]) -> Vec<String> {
    arguments
        .windows(2)
        .filter(|pair| pair[0] == "-Xv")
        .map(|pair| pair[1].clone())
        .collect()
}

fn link_native_inputs(arguments: &[String]) -> Vec<String> {
    arguments
        .iter()
        .filter(|argument| {
            std::path::Path::new(argument)
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| {
                    matches!(
                        extension.to_ascii_lowercase().as_str(),
                        "c" | "cc" | "cpp" | "cxx"
                    )
                })
        })
        .cloned()
        .collect()
}

fn simulation_file_inputs(arguments: &[String]) -> Vec<&str> {
    arguments
        .windows(2)
        .filter(|pair| pair[0] == "-f")
        .map(|pair| pair[1].as_str())
        .collect()
}

fn validate_simulation_file_options(arguments: &[String]) -> Result<(), ValidationError> {
    for (index, argument) in arguments.iter().enumerate() {
        if argument == "-f"
            && arguments
                .get(index + 1)
                .is_none_or(|path| path.starts_with('-'))
        {
            return Err(ValidationError::new(
                "simulation.run -f requires a following command-file path",
            ));
        }
    }
    Ok(())
}

fn validate_bsc_options_argv(arguments: &[String]) -> Result<(), ValidationError> {
    let mut index = 0;
    while index < arguments.len() {
        let argument = &arguments[index];
        if argument.is_empty() {
            return Err(ValidationError::new("argv entries must not be empty"));
        }
        if argument == "-p" {
            let Some(paths) = arguments.get(index + 1) else {
                return Err(ValidationError::new("bsc.options -p requires a path list"));
            };
            validate_bsc_relative_path_list(paths)?;
            index += 2;
            continue;
        }
        if argv_entry_has_unsafe_path(argument) {
            return Err(ValidationError::new(format!(
                "argv entry contains an unsafe path: {argument}"
            )));
        }
        index += 1;
    }
    Ok(())
}

fn validate_bsc_relative_path_list(paths: &str) -> Result<(), ValidationError> {
    if paths.contains(['\\', '\0']) || paths.to_ascii_lowercase().contains(".bsc-test-plan") {
        return Err(ValidationError::new(
            "bsc.options -p path list contains an unsafe path",
        ));
    }
    for path in paths.split(':') {
        let path = path.strip_prefix('+').unwrap_or(path);
        if path.is_empty() {
            continue;
        }
        if path.starts_with('/') || path.as_bytes().get(1) == Some(&b':') {
            return Err(ValidationError::new(
                "bsc.options -p path list must be relative",
            ));
        }
        let mut depth = 0usize;
        for segment in path.split('/') {
            match segment {
                "" | "." => {}
                ".." => {
                    if depth == 0 {
                        return Err(ValidationError::new(
                            "bsc.options -p path list escapes the work directory",
                        ));
                    }
                    depth -= 1;
                }
                _ => depth += 1,
            }
        }
    }
    Ok(())
}

fn validate_argv(arguments: &[String]) -> Result<(), ValidationError> {
    for argument in arguments {
        if argument.is_empty() {
            return Err(ValidationError::new("argv entries must not be empty"));
        }
        if argv_entry_has_unsafe_path(argument) {
            return Err(ValidationError::new(format!(
                "argv entry contains an unsafe path: {argument}"
            )));
        }
    }
    Ok(())
}

fn argv_entry_has_unsafe_path(argument: &str) -> bool {
    let lowercase = argument.to_ascii_lowercase();
    let has_parent = argument.split(['/', '\\']).any(|segment| segment == "..");
    let option_has_path_separator = argument.starts_with('-')
        && argument.char_indices().any(|(index, character)| {
            character == '/'
                || (character == '\\'
                    && !matches!(
                        argument[index + character.len_utf8()..].chars().next(),
                        Some('"' | '\'')
                    ))
        });
    let has_equals_root = argument.contains("=/")
        || argument
            .match_indices("=\\")
            .any(|(index, _)| !matches!(argument[index + 2..].chars().next(), Some('"' | '\'')));
    let has_drive_prefix = argument
        .as_bytes()
        .windows(2)
        .enumerate()
        .any(|(index, bytes)| {
            bytes[0].is_ascii_alphabetic()
                && bytes[1] == b':'
                && (index == 0 || argument.as_bytes()[index - 1] == b'=')
        });
    argument.contains('\0')
        || argument.starts_with(['/', '\\'])
        || option_has_path_separator
        || argument.contains("../")
        || argument.contains("..\\")
        || has_equals_root
        || has_parent
        || has_drive_prefix
        || lowercase.contains(".bsc-test-plan")
}

fn is_safe_systemc_define(define: &str) -> bool {
    define.starts_with("-D")
        && define.len() > 2
        && !define.contains(char::is_whitespace)
        && !define.contains(['/', '\\', '\0'])
}

fn validate_path(path: &str, label: &str) -> Result<(), ValidationError> {
    let mut segments = path.split('/');
    let first = segments.next().unwrap_or_default();
    let invalid = path.is_empty()
        || path.contains('\\')
        || path.starts_with('/')
        || first.as_bytes().get(1) == Some(&b':')
        || std::iter::once(first).chain(segments).any(|segment| {
            segment.is_empty()
                || segment == "."
                || segment == ".."
                || segment.eq_ignore_ascii_case(".bsc-test-plan")
        });
    if invalid {
        return Err(ValidationError::new(format!(
            "{label} must be a canonical safe relative path: {path}"
        )));
    }
    Ok(())
}

fn validate_portable_segment(segment: &str, label: &str) -> Result<(), ValidationError> {
    let invalid = segment.is_empty()
        || segment == "."
        || segment == ".."
        || segment.contains(['/', '\\'])
        || segment.eq_ignore_ascii_case(".bsc-test-plan")
        || is_windows_incompatible_segment(segment);
    if invalid {
        return Err(ValidationError::new(format!(
            "{label} must be a portable file-name segment: {segment}"
        )));
    }
    Ok(())
}

pub fn path_requires_non_windows(path: &str) -> bool {
    path.split('/').any(is_windows_incompatible_segment)
}

fn is_windows_incompatible_segment(segment: &str) -> bool {
    segment.chars().any(|character| {
        character <= '\u{1f}' || matches!(character, ':' | '<' | '>' | '"' | '|' | '?' | '*')
    }) || segment.ends_with([' ', '.'])
        || is_windows_reserved_name(segment)
}

fn is_windows_reserved_name(segment: &str) -> bool {
    let base = segment.split('.').next().unwrap_or_default();
    let bytes = base.as_bytes();
    base.eq_ignore_ascii_case("CON")
        || base.eq_ignore_ascii_case("PRN")
        || base.eq_ignore_ascii_case("AUX")
        || base.eq_ignore_ascii_case("NUL")
        || (bytes.len() == 4
            && (bytes[..3].eq_ignore_ascii_case(b"COM") || bytes[..3].eq_ignore_ascii_case(b"LPT"))
            && matches!(bytes[3], b'1'..=b'9'))
}

fn validate_hash(hash: &str, label: &str) -> Result<(), ValidationError> {
    if hash.len() != 64
        || !hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(ValidationError::new(format!(
            "{label} must be a lowercase SHA-256 digest"
        )));
    }
    Ok(())
}

fn is_safe_id(id: &str) -> bool {
    !id.contains('\\')
        && !id
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span() -> SourceSpan {
        SourceSpan {
            start_byte: 0,
            end_byte: 1,
            start_line: 1,
            start_column: 1,
            end_line: 1,
            end_column: 2,
        }
    }

    fn showrules_action() -> Action {
        Action::ShowRules {
            top: "mkTop".to_owned(),
            input: "raw.vcd".to_owned(),
            output: "rules.vcd".to_owned(),
            design_inputs: vec!["mkChild.ba".to_owned(), "mkTop.ba".to_owned()],
            stdout: "raw.vcd.showrules-out".to_owned(),
        }
    }

    #[test]
    fn flag_preflight_is_closed_and_never_declares_its_non_materialized_input() {
        let source = Action::BscFlagPreflight {
            mode: BscFlagPreflightMode::VerilogNoOptUndetermined,
            input: "NoOptUndet_UnspecToX.bsv".to_owned(),
            top: None,
            unspecified_to: UndeterminedValue::X,
            stdout: "NoOptUndet_UnspecToX.bsv.bsc-out".to_owned(),
        };
        validate_action(&source).unwrap();
        assert_eq!(
            ArtifactContract::for_action(&source),
            ArtifactContract {
                inputs: Vec::new(),
                outputs: vec!["NoOptUndet_UnspecToX.bsv.bsc-out".to_owned()],
                output_alternatives: Vec::new(),
                directories: Vec::new(),
                removes: Vec::new(),
            }
        );
        assert_eq!(
            serde_json::from_value::<Action>(serde_json::to_value(&source).unwrap()).unwrap(),
            source
        );

        let link = Action::BscFlagPreflight {
            mode: BscFlagPreflightMode::BluesimLink,
            input: "m.ba".to_owned(),
            top: Some("mkBluesimLink_UnspecToZ".to_owned()),
            unspecified_to: UndeterminedValue::Z,
            stdout: "mkBluesimLink_UnspecToZ.bsc-ccomp-out".to_owned(),
        };
        validate_action(&link).unwrap();

        let mut invalid = link.clone();
        if let Action::BscFlagPreflight { top, .. } = &mut invalid {
            *top = None;
        }
        assert!(validate_action(&invalid).is_err());
        let mut invalid = source;
        if let Action::BscFlagPreflight { input, .. } = &mut invalid {
            *input = "missing.ba".to_owned();
        }
        assert!(validate_action(&invalid).is_err());
    }

    #[test]
    fn simir_m2_run_is_closed_and_has_fixed_artifacts() {
        let action = Action::SimirM2Run {
            model: "mkMCDTest.m2.bsim.json".to_owned(),
            max_events: 100,
            expected_finish: 0,
            expected_time: 163,
            stdout: "mkMCDTest_m2_run.out".to_owned(),
        };
        validate_action(&action).unwrap();
        assert_eq!(
            ArtifactContract::for_action(&action),
            ArtifactContract {
                inputs: vec!["mkMCDTest.m2.bsim.json".to_owned()],
                outputs: vec!["mkMCDTest_m2_run.out".to_owned()],
                output_alternatives: Vec::new(),
                directories: Vec::new(),
                removes: Vec::new(),
            }
        );
        let encoded = serde_json::to_value(&action).unwrap();
        assert_eq!(encoded["op"], "simir.m2_run");
        assert_eq!(encoded["maxEvents"], 100);
        assert_eq!(encoded["expectedFinish"], 0);
        assert_eq!(encoded["expectedTime"], 163);
        assert_eq!(serde_json::from_value::<Action>(encoded).unwrap(), action);

        for action in [
            Action::SimirM2Run {
                model: "mkMCDTest.bsim.json".to_owned(),
                max_events: 100,
                expected_finish: 0,
                expected_time: 163,
                stdout: "mkMCDTest_m2_run.out".to_owned(),
            },
            Action::SimirM2Run {
                model: "mkMCDTest.m2.bsim.json".to_owned(),
                max_events: 0,
                expected_finish: 0,
                expected_time: 163,
                stdout: "mkMCDTest_m2_run.out".to_owned(),
            },
            Action::SimirM2Run {
                model: "mkMCDTest.m2.bsim.json".to_owned(),
                max_events: 1_000_001,
                expected_finish: 0,
                expected_time: 163,
                stdout: "mkMCDTest_m2_run.out".to_owned(),
            },
        ] {
            assert!(validate_action(&action).is_err(), "accepted {action:?}");
        }
    }

    #[test]
    fn simir_m3_run_is_closed_and_has_fixed_artifacts() {
        let action = Action::SimirM3Run {
            model: "mkMCDTest.m3.bsim.json".to_owned(),
            max_events: 100,
            expected_finish: 0,
            expected_time: 163,
            stdout: "mkMCDTest_m3_run.out".to_owned(),
        };
        validate_action(&action).unwrap();
        assert_eq!(
            ArtifactContract::for_action(&action),
            ArtifactContract {
                inputs: vec!["mkMCDTest.m3.bsim.json".to_owned()],
                outputs: vec!["mkMCDTest_m3_run.out".to_owned()],
                output_alternatives: Vec::new(),
                directories: Vec::new(),
                removes: Vec::new(),
            }
        );
        let encoded = serde_json::to_value(&action).unwrap();
        assert_eq!(encoded["op"], "simir.m3_run");
        assert_eq!(encoded["maxEvents"], 100);
        assert_eq!(encoded["expectedFinish"], 0);
        assert_eq!(encoded["expectedTime"], 163);
        assert_eq!(serde_json::from_value::<Action>(encoded).unwrap(), action);

        for action in [
            Action::SimirM3Run {
                model: "mkMCDTest.bsim.json".to_owned(),
                max_events: 100,
                expected_finish: 0,
                expected_time: 163,
                stdout: "mkMCDTest_m3_run.out".to_owned(),
            },
            Action::SimirM3Run {
                model: "mkMCDTest.m3.bsim.json".to_owned(),
                max_events: 0,
                expected_finish: 0,
                expected_time: 163,
                stdout: "mkMCDTest_m3_run.out".to_owned(),
            },
            Action::SimirM3Run {
                model: "mkMCDTest.m3.bsim.json".to_owned(),
                max_events: 1_000_001,
                expected_finish: 0,
                expected_time: 163,
                stdout: "mkMCDTest_m3_run.out".to_owned(),
            },
        ] {
            assert!(validate_action(&action).is_err(), "accepted {action:?}");
        }
    }

    #[test]
    fn showrules_action_is_closed_and_has_fixed_artifacts() {
        let action = showrules_action();
        validate_action(&action).unwrap();
        assert_eq!(
            ArtifactContract::for_action(&action),
            ArtifactContract {
                inputs: vec![
                    "raw.vcd".to_owned(),
                    "mkChild.ba".to_owned(),
                    "mkTop.ba".to_owned(),
                ],
                outputs: vec!["rules.vcd".to_owned(), "raw.vcd.showrules-out".to_owned()],
                output_alternatives: Vec::new(),
                directories: Vec::new(),
                removes: Vec::new(),
            }
        );
        let encoded = serde_json::to_value(&action).unwrap();
        assert_eq!(encoded["op"], "vcd.showrules");
        assert_eq!(encoded["designInputs"][1], "mkTop.ba");
        assert!(encoded.get("args").is_none());
        assert!(encoded.get("options").is_none());
    }

    #[test]
    fn showrules_action_rejects_options_by_schema_and_unsafe_or_ambiguous_fields() {
        let mut encoded = serde_json::to_value(showrules_action()).unwrap();
        encoded["options"] = serde_json::json!(["--verbose"]);
        assert!(serde_json::from_value::<Action>(encoded).is_err());

        let mutate = |field: &str, value: Vec<String>| {
            let mut action = showrules_action();
            let Action::ShowRules {
                top,
                input,
                output,
                design_inputs,
                stdout,
            } = &mut action
            else {
                unreachable!()
            };
            match field {
                "top" => *top = value[0].clone(),
                "input" => {
                    *input = value[0].clone();
                    *stdout = format!("{}.showrules-out", value[0]);
                }
                "output" => *output = value[0].clone(),
                "designInputs" => *design_inputs = value,
                _ => unreachable!(),
            }
            action
        };
        let invalid = [
            mutate("top", vec!["../mkTop".to_owned()]),
            mutate("output", vec!["RAW.VCD".to_owned()]),
            mutate("input", vec!["raw.txt".to_owned()]),
            mutate("designInputs", Vec::new()),
            mutate("designInputs", vec!["mkChild.ba".to_owned()]),
            mutate(
                "designInputs",
                vec!["mkTop.ba".to_owned(), "MKTOP.BA".to_owned()],
            ),
            mutate(
                "designInputs",
                vec!["mkTop.ba".to_owned(), "../mkChild.ba".to_owned()],
            ),
        ];
        for action in invalid {
            assert!(validate_action(&action).is_err(), "accepted {action:?}");
        }
    }

    fn complete_plan() -> TestPlan {
        TestPlan {
            schema_version: TEST_PLAN_SCHEMA_VERSION,
            id: "bsc.bluesim/parallel/parallel".to_owned(),
            origin: Origin {
                path: "testsuite/bsc.bluesim/parallel/parallel.exp".to_owned(),
                sha256: "0".repeat(64),
            },
            status: PlanStatus::Complete,
            fixture_dir: "testsuite/bsc.bluesim/parallel".to_owned(),
            fixtures: vec![Fixture {
                path: "GCD.bsv".to_owned(),
                source: None,
                sha256: "1".repeat(64),
                role: FixtureRole::Source,
            }],
            scenarios: vec![Scenario {
                id: "incremental-link".to_owned(),
                resource: ResourceClass::Heavy,
                fixtures: vec!["GCD.bsv".to_owned()],
                requires: vec![Requirement::Bluesim],
                bsc_options_append: None,
                timeouts: Timeouts::default(),
                stages: vec![Stage {
                    id: "mkGCD".to_owned(),
                    operations: vec![OperationRecord::new(
                        Action::BscGenerate {
                            source: "GCD.bsv".to_owned(),
                            mode: SimulationGenerationMode::Bluesim,
                            module: Some("mkGCD".to_owned()),
                            args: Vec::new(),
                        },
                        OperationExpectation::Required,
                        Provenance {
                            span: span(),
                            expansion: Vec::new(),
                        },
                    )],
                }],
            }],
            diagnostics: Vec::new(),
        }
    }

    #[test]
    fn make_test_data_action_has_fixed_artifact_contracts() {
        let action = Action::MakeTestData;
        let rendered = serde_json::to_value(&action).unwrap();
        assert_eq!(rendered["op"], "upstream.make_test_data");
        assert_eq!(serde_json::from_value::<Action>(rendered).unwrap(), action);
        let artifacts = ArtifactContract::for_action(&action);
        assert_eq!(artifacts.inputs, ["Makefile.data", "dumper.c"]);
        assert_eq!(
            artifacts.outputs,
            [
                "testa.dat",
                "testm.dat",
                "testmac.dat",
                "testa64.dat",
                "testm64.dat",
                "testmac64.dat",
            ]
        );
        assert_eq!(artifacts.output_alternatives, [["dumper", "dumper.exe"]]);
    }

    #[test]
    fn systemc_actions_serialize_as_typed_operations() {
        let actions = [
            Action::BscSystemcLink {
                objects: vec!["mkTop.ba".to_owned()],
                top: "mkTop".to_owned(),
                expected_exit: ExpectedExit::Success,
            },
            Action::SystemcCxxLink {
                executable: "top".to_owned(),
                sources: vec!["main.cpp".to_owned()],
                top_modules: vec!["mkTop".to_owned()],
                other_modules: vec!["mkHelper".to_owned()],
                defines: vec!["-DENABLE=1".to_owned()],
            },
            Action::SystemcRun {
                executable: "top".to_owned(),
                stdout: "top.out".to_owned(),
                sort_output: true,
            },
        ];

        for (action, operation) in
            actions
                .iter()
                .zip(["bsc.systemc_link", "systemc.cxx_link", "systemc.run"])
        {
            let rendered = serde_json::to_value(action).unwrap();
            assert_eq!(rendered["op"], operation);
            assert_eq!(serde_json::from_value::<Action>(rendered).unwrap(), *action);
        }
    }

    #[test]
    fn systemc_actions_declare_typed_artifact_contracts() {
        let bsc_link = Action::BscSystemcLink {
            objects: vec!["build/mkTop.ba".to_owned()],
            top: "mkTop".to_owned(),
            expected_exit: ExpectedExit::Success,
        };
        assert_eq!(
            ArtifactContract::for_action(&bsc_link).inputs,
            ["build/mkTop.ba"]
        );
        assert_eq!(
            ArtifactContract::for_action(&bsc_link).outputs,
            [
                "mkTop.bsc-ccomp-out",
                "mkTop.o",
                "mkTop_systemc.o",
                "model_mkTop.o"
            ]
        );

        let cxx_link = Action::SystemcCxxLink {
            executable: "top".to_owned(),
            sources: vec!["main.cpp".to_owned(), "support.cpp".to_owned()],
            top_modules: vec!["mkTop".to_owned()],
            other_modules: vec!["mkHelper".to_owned()],
            defines: vec!["-DENABLE=1".to_owned()],
        };
        assert_eq!(
            ArtifactContract::for_action(&cxx_link).inputs,
            [
                "main.cpp",
                "support.cpp",
                "mkTop.o",
                "mkTop_systemc.o",
                "mkHelper.o",
                "mkHelper_systemc.o",
                "model_mkTop.o",
            ]
        );
        assert_eq!(
            ArtifactContract::for_action(&cxx_link).outputs,
            ["top.syscexe", "top.cxx-comp-out"]
        );

        let run = Action::SystemcRun {
            executable: "top".to_owned(),
            stdout: "top.out".to_owned(),
            sort_output: true,
        };
        assert_eq!(ArtifactContract::for_action(&run).inputs, ["top.syscexe"]);
        assert_eq!(
            ArtifactContract::for_action(&run).outputs,
            ["top.out", "top.raw.out"]
        );
    }

    #[test]
    fn bluetcl_actions_are_closed_typed_and_require_the_bluetcl_capability() {
        let action = Action::BluetclRun {
            invocation: BluetclInvocation::Exec {
                script: "dump_poss.tcl".to_owned(),
                args: vec!["sysTop".to_owned()],
            },
            working_directory: None,
            artifact_inputs: vec!["sysTop.ba".to_owned()],
            artifact_outputs: Vec::new(),
            expected_exit: ExpectedExit::Failure,
            stdout: "sysTop.bluetcl-out".to_owned(),
        };
        let operation = OperationRecord::new(
            action.clone(),
            OperationExpectation::Required,
            Provenance {
                span: span(),
                expansion: Vec::new(),
            },
        );
        assert_eq!(operation.artifacts.inputs, ["dump_poss.tcl", "sysTop.ba"]);
        assert_eq!(operation.artifacts.outputs, ["sysTop.bluetcl-out"]);
        let decoded: Action =
            serde_json::from_value(serde_json::to_value(&action).unwrap()).unwrap();
        assert_eq!(decoded, action);

        for script in ["not-tcl", "../dump_poss.tcl"] {
            let invalid = Action::BluetclRun {
                invocation: BluetclInvocation::Script {
                    script: script.to_owned(),
                    args: Vec::new(),
                    syntax: BluetclSyntax::Bsv,
                },
                working_directory: None,
                artifact_inputs: Vec::new(),
                artifact_outputs: Vec::new(),
                expected_exit: ExpectedExit::Success,
                stdout: "output.bluetcl-out".to_owned(),
            };
            assert!(validate_action(&invalid).is_err(), "{script}");
        }

        let mut plan = complete_plan();
        plan.fixtures = vec![Fixture {
            path: "utils_test.tcl".to_owned(),
            source: None,
            sha256: "1".repeat(64),
            role: FixtureRole::Script,
        }];
        plan.scenarios[0].fixtures = vec!["utils_test.tcl".to_owned()];
        plan.scenarios[0].requires = vec![Requirement::Bluetcl];
        plan.scenarios[0].stages[0].operations = vec![OperationRecord::new(
            Action::BluetclRun {
                invocation: BluetclInvocation::Script {
                    script: "utils_test.tcl".to_owned(),
                    args: Vec::new(),
                    syntax: BluetclSyntax::Bh,
                },
                working_directory: None,
                artifact_inputs: Vec::new(),
                artifact_outputs: Vec::new(),
                expected_exit: ExpectedExit::Success,
                stdout: "utils.out".to_owned(),
            },
            OperationExpectation::Required,
            Provenance {
                span: span(),
                expansion: Vec::new(),
            },
        )];
        plan.validate().unwrap();
        plan.scenarios[0].requires.clear();
        assert!(plan.validate().is_err());
    }

    #[test]
    fn package_requirements_are_operation_scoped_and_installed_scripts_are_exact() {
        let action = Action::BluetclRun {
            invocation: BluetclInvocation::InstalledScript {
                script: BluetclInstalledScript::ExpandPorts,
                args: vec![
                    "-quiet".to_owned(),
                    "-rename".to_owned(),
                    "Test7b.rename.tcl".to_owned(),
                    "-wrapper".to_owned(),
                    "mkTest7b.wrapper.got.v".to_owned(),
                    "-include".to_owned(),
                    "mkTest7b.includes.got.vh".to_owned(),
                    "Test7b".to_owned(),
                    "mkTest7b".to_owned(),
                    "mkTest7b.v".to_owned(),
                ],
            },
            working_directory: None,
            artifact_inputs: vec![
                "Test7b.bo".to_owned(),
                "mkTest7b.ba".to_owned(),
                "mkTest7b.v".to_owned(),
                "Test7b.rename.tcl".to_owned(),
            ],
            artifact_outputs: vec![
                "mkTest7b.wrapper.got.v".to_owned(),
                "mkTest7b.includes.got.vh".to_owned(),
            ],
            expected_exit: ExpectedExit::Success,
            stdout: "Test7b.expandPorts.bluetcl-out".to_owned(),
        };
        validate_action(&action).unwrap();
        let artifacts = ArtifactContract::for_action(&action);
        assert!(artifacts
            .outputs
            .contains(&"mkTest7b.wrapper.got.v".to_owned()));
        assert!(artifacts
            .outputs
            .contains(&"mkTest7b.includes.got.vh".to_owned()));

        let mut operation = OperationRecord::new(
            action.clone(),
            OperationExpectation::Required,
            Provenance {
                span: span(),
                expansion: Vec::new(),
            },
        );
        assert!(validate_operation(&operation).is_err());
        operation
            .requires
            .push(Requirement::BluetclPackage(BluetclPackage::ExpandPorts));
        validate_operation(&operation).unwrap();

        for invalid in [
            {
                let mut invalid = action.clone();
                let Action::BluetclRun { invocation, .. } = &mut invalid else {
                    unreachable!()
                };
                let BluetclInvocation::InstalledScript { args, .. } = invocation else {
                    unreachable!()
                };
                args.push("extra".to_owned());
                invalid
            },
            {
                let mut invalid = action.clone();
                let Action::BluetclRun {
                    artifact_outputs, ..
                } = &mut invalid
                else {
                    unreachable!()
                };
                artifact_outputs.pop();
                invalid
            },
            {
                let mut invalid = action.clone();
                let Action::BluetclRun { expected_exit, .. } = &mut invalid else {
                    unreachable!()
                };
                *expected_exit = ExpectedExit::Failure;
                invalid
            },
        ] {
            assert!(validate_action(&invalid).is_err());
        }

        let mut plan = complete_plan();
        plan.scenarios[0]
            .requires
            .push(Requirement::BluetclPackage(BluetclPackage::InstSynth));
        assert!(plan.validate().is_err());
    }

    #[test]
    fn makedepend_invocations_bind_argv_workdir_exit_and_artifacts() {
        let inputs = [
            "Dep1.bsv",
            "Foo.bsv",
            "IncDep1.bsv",
            "IncDep2.bsv",
            "Test.bsv",
            "include1.inc",
            "subinclude.inc",
        ]
        .into_iter()
        .map(|path| format!("makedepend/{path}"))
        .collect::<Vec<_>>();
        let action = Action::BluetclRun {
            invocation: BluetclInvocation::Makedepend {
                command: BluetclMakedependCommand::Makedepend,
                args: vec![
                    "-no-show-timestamps".to_owned(),
                    "-bdir".to_owned(),
                    "objs".to_owned(),
                    "-p".to_owned(),
                    "../makedepend/:%/Libraries".to_owned(),
                    "-D".to_owned(),
                    "INC1".to_owned(),
                    "Dep1.bsv".to_owned(),
                ],
            },
            working_directory: Some("makedepend".to_owned()),
            artifact_inputs: inputs,
            artifact_outputs: Vec::new(),
            expected_exit: ExpectedExit::Success,
            stdout: "makedepend/updir.bluetcl-out".to_owned(),
        };
        validate_action(&action).unwrap();

        for invalid in [
            {
                let mut invalid = action.clone();
                let Action::BluetclRun {
                    working_directory, ..
                } = &mut invalid
                else {
                    unreachable!()
                };
                *working_directory = None;
                invalid
            },
            {
                let mut invalid = action.clone();
                let Action::BluetclRun { invocation, .. } = &mut invalid else {
                    unreachable!()
                };
                let BluetclInvocation::Makedepend { args, .. } = invocation else {
                    unreachable!()
                };
                args[7] = "Dep2.bsv".to_owned();
                invalid
            },
            {
                let mut invalid = action.clone();
                let Action::BluetclRun { stdout, .. } = &mut invalid else {
                    unreachable!()
                };
                *stdout = "updir.bluetcl-out".to_owned();
                invalid
            },
            {
                let mut invalid = action.clone();
                let Action::BluetclRun {
                    artifact_inputs, ..
                } = &mut invalid
                else {
                    unreachable!()
                };
                artifact_inputs.pop();
                invalid
            },
        ] {
            assert!(validate_action(&invalid).is_err());
        }
    }

    #[test]
    fn systemc_actions_reject_unsafe_or_incomplete_inputs() {
        for action in [
            Action::BscSystemcLink {
                objects: Vec::new(),
                top: "mkTop".to_owned(),
                expected_exit: ExpectedExit::Success,
            },
            Action::BscSystemcLink {
                objects: vec!["mkTop.o".to_owned()],
                top: "mkTop".to_owned(),
                expected_exit: ExpectedExit::Success,
            },
            Action::SystemcCxxLink {
                executable: "top".to_owned(),
                sources: Vec::new(),
                top_modules: vec!["mkTop".to_owned()],
                other_modules: Vec::new(),
                defines: Vec::new(),
            },
            Action::SystemcCxxLink {
                executable: "top".to_owned(),
                sources: vec!["main.cpp".to_owned()],
                top_modules: Vec::new(),
                other_modules: Vec::new(),
                defines: Vec::new(),
            },
            Action::SystemcCxxLink {
                executable: "top".to_owned(),
                sources: vec!["main.cpp".to_owned()],
                top_modules: vec!["../mkTop".to_owned()],
                other_modules: Vec::new(),
                defines: Vec::new(),
            },
            Action::SystemcCxxLink {
                executable: "top".to_owned(),
                sources: vec!["main.cpp".to_owned()],
                top_modules: vec!["mkTop".to_owned()],
                other_modules: Vec::new(),
                defines: vec!["-D BAD".to_owned()],
            },
            Action::SystemcCxxLink {
                executable: "top".to_owned(),
                sources: vec!["../main.cpp".to_owned()],
                top_modules: vec!["mkTop".to_owned()],
                other_modules: Vec::new(),
                defines: vec!["-DOUT=../outside".to_owned()],
            },
        ] {
            assert!(validate_action(&action).is_err(), "{action:?}");
        }
    }

    #[test]
    fn systemc_actions_require_declared_capabilities() {
        let mut plan = complete_plan();
        plan.scenarios[0].stages[0].operations = vec![OperationRecord::new(
            Action::SystemcRun {
                executable: "top".to_owned(),
                stdout: "top.out".to_owned(),
                sort_output: false,
            },
            OperationExpectation::Required,
            Provenance {
                span: span(),
                expansion: Vec::new(),
            },
        )];
        assert!(plan.validate().is_err());

        plan.scenarios[0].requires = vec![Requirement::SystemC];
        assert!(plan.validate().is_err());
        plan.scenarios[0].fixtures.push("top.syscexe".to_owned());
        plan.fixtures.push(Fixture {
            path: "top.syscexe".to_owned(),
            source: None,
            sha256: "2".repeat(64),
            role: FixtureRole::BuildInput,
        });
        plan.validate().unwrap();

        plan.scenarios[0].stages[0].operations = vec![OperationRecord::new(
            Action::BscSystemcLink {
                objects: vec!["mkTop.ba".to_owned()],
                top: "mkTop".to_owned(),
                expected_exit: ExpectedExit::Success,
            },
            OperationExpectation::Required,
            Provenance {
                span: span(),
                expansion: Vec::new(),
            },
        )];
        plan.scenarios[0].fixtures = vec!["mkTop.ba".to_owned()];
        plan.fixtures
            .retain(|fixture| fixture.path != "top.syscexe");
        plan.fixtures.push(Fixture {
            path: "mkTop.ba".to_owned(),
            source: None,
            sha256: "2".repeat(64),
            role: FixtureRole::BuildInput,
        });
        assert!(plan.validate().is_err());
        plan.scenarios[0].requires.push(Requirement::Bluesim);
        plan.validate().unwrap();
    }

    #[test]
    fn generation_mode_names_the_upstream_compiler_output() {
        assert_eq!(
            SimulationGenerationMode::Bluesim.compiler_output_path("Top.bsv"),
            "Top.bsv.bsc-ccomp-out"
        );
        assert_eq!(
            SimulationGenerationMode::Verilog.compiler_output_path("Top.bsv"),
            "Top.bsv.bsc-vcomp-out"
        );
        assert_eq!(
            SimulationGenerationMode::SharedElaboration.compiler_output_path("Top.bsv"),
            "Top.bsv.bsc-vcomp-out"
        );
    }

    #[test]
    fn rendered_golden_declares_template_input_and_rejects_windows_collision() {
        let action = Action::RenderGolden {
            template: "flags.expected".to_owned(),
            output: "flags.rendered".to_owned(),
            replacement: GoldenReplacement::BluespecDir,
        };
        assert_eq!(
            ArtifactContract::for_action(&action).inputs,
            ["flags.expected"]
        );
        assert_eq!(
            ArtifactContract::for_action(&action).outputs,
            ["flags.rendered"]
        );

        let mut plan = complete_plan();
        plan.scenarios[0].stages[0].operations = vec![OperationRecord::new(
            Action::RenderGolden {
                template: "Flags.expected".to_owned(),
                output: "flags.expected".to_owned(),
                replacement: GoldenReplacement::BluespecDir,
            },
            OperationExpectation::Required,
            Provenance {
                span: span(),
                expansion: Vec::new(),
            },
        )];
        assert!(plan
            .validate()
            .unwrap_err()
            .to_string()
            .contains("must not collide on Windows"));
    }

    #[test]
    fn interra_operator_vectors_declare_the_closed_generator_artifacts() {
        let action = Action::InterraOperatorVectors {
            suite: InterraOperatorSuite::Logic,
        };
        let artifacts = ArtifactContract::for_action(&action);
        assert_eq!(
            artifacts.inputs,
            [
                "generate/gen.pl",
                "generate/sort.pl",
                "generate/top_code",
                "generate/bot_code",
            ]
        );
        assert_eq!(
            artifacts.outputs,
            [
                "generate/gen_logic.v",
                "generate/a.out",
                "generate/vectors",
                "generate/Vectors.bsv",
                "Vectors.bsv",
            ]
        );
        assert_eq!(InterraOperatorSuite::Logic.verilog_top(), "gen_logical");
        validate_action(&action).unwrap();
    }

    #[test]
    fn simulation_command_files_are_declared_artifact_inputs() {
        let action = Action::SimulationRun {
            backend: SimulationBackend::Bluesim,
            executable: "mkTest".to_owned(),
            args: vec!["-f".to_owned(), "step.cmd".to_owned()],
            stdout: "mkTest.out".to_owned(),
            expected_exits: ExpectedExitSet::default(),
            vcd: None,
        };
        assert_eq!(
            ArtifactContract::for_action(&action).inputs,
            ["mkTest.cexe", "step.cmd"]
        );
        validate_action(&action).unwrap();

        let invalid = Action::SimulationRun {
            backend: SimulationBackend::Bluesim,
            executable: "mkTest".to_owned(),
            args: vec!["-f".to_owned()],
            stdout: "mkTest.out".to_owned(),
            expected_exits: ExpectedExitSet::default(),
            vcd: None,
        };
        assert!(validate_action(&invalid).is_err());
    }

    #[test]
    fn generation_declares_package_artifacts_in_the_configured_bdir() {
        let action = Action::BscGenerate {
            source: "rtl/ClockDiv.bsv".to_owned(),
            mode: SimulationGenerationMode::Verilog,
            module: Some("sysClockDiv".to_owned()),
            args: vec!["-bdir".to_owned(), "build".to_owned()],
        };
        assert_eq!(
            ArtifactContract::for_action(&action).outputs,
            [
                "rtl/ClockDiv.bsv.bsc-vcomp-out",
                "build/ClockDiv.bo",
                "sysClockDiv.v",
            ]
        );
    }

    #[test]
    fn generation_declares_only_static_audited_dump_outputs() {
        let action = Action::BscGenerate {
            source: "Demo.bsv".to_owned(),
            mode: SimulationGenerationMode::SharedElaboration,
            module: Some("mkDemo".to_owned()),
            args: vec![
                "-dATS=result.ats".to_owned(),
                "-dATSexpand=%m-expanded.ats".to_owned(),
                "-dUnknown=unknown.dump".to_owned(),
                "-dsplitIf=%x.dump".to_owned(),
            ],
        };

        assert_eq!(
            ArtifactContract::for_action(&action).outputs,
            [
                "Demo.bsv.bsc-vcomp-out",
                "Demo.bo",
                "result.ats",
                "mkDemo-expanded.ats",
                "mkDemo.ba",
                "mkDemo.v",
            ]
        );
    }

    #[test]
    fn c_object_build_has_fixed_source_makefile_and_output_contract() {
        let action = Action::CObjectBuild {
            source: "convert.c".to_owned(),
            makefile: "convert.mk".to_owned(),
            output: "convert.o".to_owned(),
        };
        let artifacts = ArtifactContract::for_action(&action);
        assert_eq!(artifacts.inputs, ["convert.c", "convert.mk"]);
        assert_eq!(artifacts.outputs, ["convert.o"]);
        validate_action(&action).unwrap();

        let invalid = Action::CObjectBuild {
            source: "convert.c".to_owned(),
            makefile: "convert.mk".to_owned(),
            output: "other.o".to_owned(),
        };
        assert!(validate_action(&invalid).is_err());
    }

    #[test]
    fn generation_declares_package_artifacts_next_to_the_source_by_default() {
        let action = Action::BscGenerate {
            source: "ClockDiv.bsv".to_owned(),
            mode: SimulationGenerationMode::Verilog,
            module: Some("sysClockDiv".to_owned()),
            args: Vec::new(),
        };
        assert_eq!(
            ArtifactContract::for_action(&action).outputs,
            ["ClockDiv.bsv.bsc-vcomp-out", "ClockDiv.bo", "sysClockDiv.v",]
        );
    }

    #[test]
    fn killed_compile_dump_does_not_declare_a_package_object() {
        assert!(
            generation_package_artifacts("Demo.bsv", &["-KILLATSexpand".to_owned()]).is_empty()
        );
    }

    #[test]
    fn standard_bluesim_link_and_run_use_cexe_artifact() {
        let link = Action::BscLink {
            backend: SimulationBackend::Bluesim,
            mode: BscLinkMode::Standard,
            objects: vec!["mkTest".to_owned()],
            top: "mkTest".to_owned(),
            args: Vec::new(),
            expected_exit: ExpectedExit::Success,
            simulator: IcarusSimulatorSelector::Default,
            missing_objects: Vec::new(),
        };
        let run = Action::SimulationRun {
            backend: SimulationBackend::Bluesim,
            executable: "mkTest".to_owned(),
            args: Vec::new(),
            stdout: "mkTest.out".to_owned(),
            expected_exits: ExpectedExitSet::default(),
            vcd: None,
        };

        let contract = ArtifactContract::for_action(&link);
        assert_eq!(contract.outputs, ["mkTest.bsc-ccomp-out"]);
        assert_eq!(
            contract.output_alternatives,
            [vec!["mkTest.cexe".to_owned(), "mkTest.cexe.exe".to_owned()]]
        );
        assert_eq!(ArtifactContract::for_action(&run).inputs, ["mkTest.cexe"]);
    }

    #[test]
    fn standard_icarus_link_and_run_use_vexe_artifact() {
        let link = Action::BscLink {
            backend: SimulationBackend::Icarus,
            mode: BscLinkMode::Standard,
            objects: vec!["sysTest".to_owned()],
            top: "sysTest".to_owned(),
            args: Vec::new(),
            expected_exit: ExpectedExit::Success,
            simulator: IcarusSimulatorSelector::Default,
            missing_objects: Vec::new(),
        };
        let run = Action::SimulationRun {
            backend: SimulationBackend::Icarus,
            executable: "sysTest".to_owned(),
            args: Vec::new(),
            stdout: "sysTest.v.out".to_owned(),
            expected_exits: ExpectedExitSet::default(),
            vcd: None,
        };
        assert_eq!(
            ArtifactContract::for_action(&link).outputs,
            ["sysTest.bsc-vcomp-out", "sysTest.vexe"]
        );
        assert!(ArtifactContract::for_action(&link)
            .output_alternatives
            .is_empty());
        assert_eq!(ArtifactContract::for_action(&run).inputs, ["sysTest.vexe"]);

        let mut explicit_vcd_run = run.clone();
        let Action::SimulationRun { args, .. } = &mut explicit_vcd_run else {
            unreachable!("test action must remain a simulation run");
        };
        args.extend(["-V".to_owned(), "trace.vcd".to_owned()]);
        assert_eq!(
            ArtifactContract::for_action(&explicit_vcd_run).outputs,
            ["sysTest.v.out", "trace.vcd"]
        );

        let mut default_vcd_run = run.clone();
        let Action::SimulationRun { args, .. } = &mut default_vcd_run else {
            unreachable!("test action must remain a simulation run");
        };
        args.extend(["-V".to_owned(), "+runtime-option".to_owned()]);
        assert_eq!(
            ArtifactContract::for_action(&default_vcd_run).outputs,
            ["sysTest.v.out", "dump.vcd"]
        );

        let mut bscvcd_run = run.clone();
        let Action::SimulationRun { args, .. } = &mut bscvcd_run else {
            unreachable!("test action must remain a simulation run");
        };
        args.extend(["+bscvcd".to_owned(), "+foo".to_owned()]);
        assert_eq!(
            ArtifactContract::for_action(&bscvcd_run).outputs,
            ["sysTest.v.out"]
        );
        assert_eq!(
            simulation_vcd_outputs(&["-V".to_owned(), "trace.vcd".to_owned()]),
            ["trace.vcd"]
        );
        assert_eq!(
            simulation_vcd_outputs(&["-V".to_owned(), "+foo".to_owned()]),
            ["dump.vcd"]
        );
        let Action::SimulationRun { vcd, .. } = &mut bscvcd_run else {
            unreachable!("test action must remain a simulation run");
        };
        *vcd = Some("dump.vcd".to_owned());
        assert_eq!(
            ArtifactContract::for_action(&bscvcd_run).outputs,
            ["sysTest.v.out", "dump.vcd"]
        );

        let no_main = Action::BscLink {
            backend: SimulationBackend::Icarus,
            mode: BscLinkMode::NoMain,
            objects: vec!["sysTest".to_owned()],
            top: "sysTest".to_owned(),
            args: Vec::new(),
            expected_exit: ExpectedExit::Success,
            simulator: IcarusSimulatorSelector::Default,
            missing_objects: Vec::new(),
        };
        assert_eq!(
            ArtifactContract::for_action(&no_main).outputs,
            ["sysTest.bsc-vcomp-out", "sysTest.vexe"]
        );
    }

    #[test]
    fn expected_exit_sets_select_architecture_specific_codes() {
        let exits = ExpectedExitSet::new(vec![8, 136], Some(vec![0]), Some(vec![127]));
        assert!(exits.accepts_for_platform(8, false, false));
        assert!(exits.accepts_for_platform(136, false, false));
        assert!(!exits.accepts_for_platform(0, false, false));
        assert!(exits.accepts_for_platform(0, true, false));
        assert!(!exits.accepts_for_platform(8, true, false));
        assert!(exits.accepts_for_platform(127, false, true));
        assert!(!exits.accepts_for_platform(8, false, true));
        exits.validate().unwrap();

        assert!(ExpectedExitSet::new(vec![8, 8], None, None)
            .validate()
            .is_err());
        assert!(ExpectedExitSet {
            codes: vec![8],
            aarch64_codes: Some(Vec::new()),
            windows_codes: None,
        }
        .validate()
        .is_err());
    }

    #[test]
    fn xfail_expectations_are_limited_to_compile_link_and_assertion_contracts() {
        let provenance = Provenance {
            span: span(),
            expansion: Vec::new(),
        };
        validate_operation(&OperationRecord::new(
            Action::AssertDiagnosticCount {
                path: "compile.out".to_owned(),
                kind: DiagnosticKind::Warning,
                code: None,
                count: 0,
            },
            OperationExpectation::Xfail {
                reason: "upstream bug 1".to_owned(),
            },
            provenance.clone(),
        ))
        .unwrap();
        validate_operation(&OperationRecord::new(
            Action::AssertRegexAbsent {
                path: "compile.out".to_owned(),
                pattern: "Internal.*Error".to_owned(),
            },
            OperationExpectation::Xfail {
                reason: "upstream bug 1".to_owned(),
            },
            provenance.clone(),
        ))
        .unwrap();
        assert!(validate_operation(&OperationRecord::new(
            Action::FsRemove {
                path: "compile.out".to_owned(),
            },
            OperationExpectation::Xfail {
                reason: "upstream bug 1".to_owned(),
            },
            provenance,
        ))
        .is_err());
    }

    #[test]
    fn rejects_empty_or_multiline_bsc_options_append() {
        for append in ["", "  ", "-D FOO\n-D BAR"] {
            let mut plan = complete_plan();
            plan.scenarios[0].bsc_options_append = Some(append.to_owned());
            assert!(
                plan.validate().is_err(),
                "append {append:?} must be rejected"
            );
        }

        let mut plan = complete_plan();
        plan.scenarios[0].bsc_options_append = Some("-D FOO".to_owned());
        plan.validate().unwrap();
    }

    #[test]
    fn bsc2bsv_action_is_closed_and_allows_operation_internal_checks() {
        let action = Action::Bsc2Bsv {
            source: "Bug611.bs".to_owned(),
            stdout: "Bug611.bs.bsc2bsv-out".to_owned(),
        };
        assert_eq!(
            ArtifactContract::for_action(&action),
            ArtifactContract {
                inputs: vec!["Bug611.bs".to_owned()],
                outputs: vec!["Bug611.bs.bsc2bsv-out".to_owned()],
                output_alternatives: Vec::new(),
                directories: Vec::new(),
                removes: Vec::new(),
            }
        );
        let mut operation = OperationRecord::new(
            action,
            OperationExpectation::Required,
            Provenance {
                span: span(),
                expansion: Vec::new(),
            },
        );
        operation.requires.push(Requirement::InternalChecks);
        validate_operation(&operation).unwrap();

        let invalid = Action::Bsc2Bsv {
            source: "Bug611.bs".to_owned(),
            stdout: "other.out".to_owned(),
        };
        assert!(validate_action(&invalid).is_err());
    }

    #[test]
    fn delay_and_copy_replace_are_closed_typed_actions() {
        let delay = Action::Delay {
            milliseconds: 1_500,
        };
        assert_eq!(
            ArtifactContract::for_action(&delay),
            ArtifactContract {
                inputs: Vec::new(),
                outputs: Vec::new(),
                output_alternatives: Vec::new(),
                directories: Vec::new(),
                removes: Vec::new(),
            }
        );
        validate_action(&delay).unwrap();
        assert!(validate_action(&Action::Delay { milliseconds: 0 }).is_err());
        assert!(validate_action(&Action::Delay {
            milliseconds: 10_001
        })
        .is_err());

        let replace = Action::FsCopyReplace {
            source: "FiveB.bs".to_owned(),
            destination: "Five.bs".to_owned(),
        };
        assert_eq!(
            ArtifactContract::for_action(&replace),
            ArtifactContract {
                inputs: vec!["FiveB.bs".to_owned(), "Five.bs".to_owned()],
                outputs: vec!["Five.bs".to_owned()],
                output_alternatives: Vec::new(),
                directories: Vec::new(),
                removes: Vec::new(),
            }
        );
        validate_action(&replace).unwrap();
    }

    #[test]
    fn parse_pretty_action_is_closed_and_allows_xfail() {
        let action = Action::BscParsePretty {
            source: "Demo.bsv".to_owned(),
            args: vec!["-p".to_owned(), "+:lib".to_owned()],
            pretty_output: "Demo.bsv-pretty-out.bsv".to_owned(),
        };
        assert_eq!(
            ArtifactContract::for_action(&action),
            ArtifactContract {
                inputs: vec!["Demo.bsv".to_owned()],
                outputs: vec![
                    "Demo.bsv-pretty-out.bsv".to_owned(),
                    "Demo.bsv.bsc-out".to_owned(),
                    "Demo.bsv-pretty-out.bsv.bsc-out".to_owned(),
                ],
                output_alternatives: Vec::new(),
                directories: Vec::new(),
                removes: Vec::new(),
            }
        );
        validate_operation(&OperationRecord::new(
            action,
            OperationExpectation::Xfail {
                reason: "upstream bug 1".to_owned(),
            },
            Provenance {
                span: span(),
                expansion: Vec::new(),
            },
        ))
        .unwrap();

        for invalid in [
            Action::BscParsePretty {
                source: "Demo.txt".to_owned(),
                args: Vec::new(),
                pretty_output: "Demo.txt-pretty-out.txt".to_owned(),
            },
            Action::BscParsePretty {
                source: "Demo.bsv".to_owned(),
                args: Vec::new(),
                pretty_output: "other.bsv".to_owned(),
            },
        ] {
            assert!(validate_action(&invalid).is_err());
        }
    }

    #[test]
    fn validates_and_round_trips_a_complete_plan() {
        let plan = complete_plan();
        plan.validate().unwrap();
        let rendered = render_plan(&plan).unwrap();
        let decoded: TestPlan = serde_json::from_str(&rendered).unwrap();
        assert_eq!(decoded, plan);
    }

    #[test]
    fn validates_fixture_alias_schema_and_rejects_self_or_non_source_aliases() {
        let mut plan = complete_plan();
        plan.fixtures.push(Fixture {
            path: "Cpreprocess1.bsv".to_owned(),
            source: Some("Cpreprocess.bsv".to_owned()),
            sha256: "2".repeat(64),
            role: FixtureRole::Source,
        });
        plan.validate().unwrap();

        let alias = plan.fixtures.last_mut().unwrap();
        alias.source = Some(alias.path.clone());
        assert!(plan.validate().is_err());

        let mut plan = complete_plan();
        plan.fixtures.push(Fixture {
            path: "Cpreprocess1.bsv".to_owned(),
            source: Some("Cpreprocess.bsv".to_owned()),
            sha256: "2".repeat(64),
            role: FixtureRole::Golden,
        });
        assert!(plan.validate().is_err());
    }

    #[test]
    fn audited_cpp_rewrite_requires_darwin_and_move_replace_can_share_its_guard() {
        let provenance = Provenance {
            span: span(),
            expansion: Vec::new(),
        };
        let mut rewrite = OperationRecord::new(
            Action::FsRewriteDarwinCppIncludePath {
                source: "raw.out".to_owned(),
                destination: "filtered.out".to_owned(),
            },
            OperationExpectation::Required,
            provenance.clone(),
        );
        assert!(validate_operation(&rewrite).is_err());
        rewrite.requires.push(Requirement::Darwin);
        validate_operation(&rewrite).unwrap();

        let mut move_replace = OperationRecord::new(
            Action::FsMoveReplace {
                source: "filtered.out".to_owned(),
                destination: "raw.out".to_owned(),
            },
            OperationExpectation::Required,
            provenance,
        );
        validate_operation(&move_replace).unwrap();
        move_replace.requires.push(Requirement::Darwin);
        validate_operation(&move_replace).unwrap();
    }

    #[test]
    fn rejects_shell_like_unsafe_paths_and_unexplained_blocked_plans() {
        let mut plan = complete_plan();
        plan.fixture_dir = "../outside".to_owned();
        assert!(plan.validate().is_err());

        let mut plan = complete_plan();
        plan.status = PlanStatus::Blocked;
        assert!(plan.validate().is_err());

        let mut plan = complete_plan();
        plan.status = PlanStatus::Disabled;
        plan.scenarios.clear();
        assert!(plan.validate().is_err());
        plan.diagnostics.push(ImportDiagnostic {
            severity: DiagnosticSeverity::Warning,
            code: "import.disabled".to_owned(),
            message: "disabled upstream".to_owned(),
            provenance: Provenance {
                span: SourceSpan {
                    start_byte: 0,
                    end_byte: 0,
                    start_line: 1,
                    start_column: 1,
                    end_line: 1,
                    end_column: 1,
                },
                expansion: Vec::new(),
            },
        });
        plan.validate().unwrap();

        let mut duplicate_diagnostic = plan.clone();
        duplicate_diagnostic
            .diagnostics
            .push(duplicate_diagnostic.diagnostics[0].clone());
        assert!(duplicate_diagnostic.validate().is_err());

        let mut disabled_with_scenarios = complete_plan();
        disabled_with_scenarios.status = PlanStatus::Disabled;
        disabled_with_scenarios.diagnostics = plan.diagnostics.clone();
        assert!(disabled_with_scenarios.validate().is_err());

        let mut plan = complete_plan();
        plan.scenarios[0].stages[0].operations[0].action = Action::FsRemove {
            path: ".bsc-test-plan/assertions/0".to_owned(),
        };
        assert!(plan.validate().is_err());

        let mut plan = complete_plan();
        plan.fixtures.clear();
        assert!(plan.validate().is_err());
    }

    #[test]
    fn scenario_fixtures_are_required_safe_unique_and_registered() {
        let mut rendered = serde_json::to_value(complete_plan()).unwrap();
        rendered["scenarios"][0]
            .as_object_mut()
            .unwrap()
            .remove("fixtures");
        assert!(serde_json::from_value::<TestPlan>(rendered).is_err());

        let mut plan = complete_plan();
        plan.scenarios[0].fixtures = vec!["../outside.bsv".to_owned()];
        assert!(plan.validate().is_err());

        let mut plan = complete_plan();
        plan.scenarios[0].fixtures.push("GCD.bsv".to_owned());
        assert!(plan.validate().is_err());

        let mut plan = complete_plan();
        plan.scenarios[0].fixtures = vec!["Other.bsv".to_owned()];
        assert!(plan.validate().is_err());
    }

    #[test]
    fn frontend_and_verilog_requirements_are_mutually_exclusive() {
        let mut plan = complete_plan();
        plan.scenarios[0].requires.push(Requirement::Frontend);
        plan.scenarios[0].requires.push(Requirement::Verilog);
        assert!(plan.validate().is_err());
    }

    #[test]
    fn golden_any_requires_nonempty_unique_expected_paths() {
        let action = |expected: Vec<&str>| Action::AssertGoldenAny {
            actual: "actual.out".to_owned(),
            expected: expected.into_iter().map(str::to_owned).collect(),
        };
        assert!(validate_action(&action(Vec::new())).is_err());
        assert!(validate_action(&action(vec!["actual.out"])).is_err());
        assert!(validate_action(&action(vec!["one.expected", "ONE.expected"])).is_err());
        let action = action(vec!["one.expected", "two.expected"]);
        assert_eq!(action.expected_paths(), ["one.expected", "two.expected"]);
        assert!(validate_action(&action).is_ok());
    }

    #[test]
    fn complete_scenarios_declare_all_direct_fixture_inputs() {
        let mut plan = complete_plan();
        plan.scenarios[0].fixtures.clear();
        assert!(plan.validate().is_err());

        let mut plan = complete_plan();
        plan.fixtures.push(Fixture {
            path: "GCD.out.expected".to_owned(),
            source: None,
            sha256: "2".repeat(64),
            role: FixtureRole::Golden,
        });
        plan.scenarios[0].stages[0]
            .operations
            .push(OperationRecord::new(
                Action::AssertGolden {
                    actual: "GCD.out".to_owned(),
                    expected: "GCD.out.expected".to_owned(),
                },
                OperationExpectation::Required,
                Provenance {
                    span: span(),
                    expansion: Vec::new(),
                },
            ));
        assert!(plan.validate().is_err());
        plan.scenarios[0]
            .fixtures
            .push("GCD.out.expected".to_owned());
        assert!(plan.validate().is_err());
        plan.fixtures.push(Fixture {
            path: "GCD.out".to_owned(),
            source: None,
            sha256: "3".repeat(64),
            role: FixtureRole::Data,
        });
        plan.scenarios[0].fixtures.push("GCD.out".to_owned());
        plan.validate().unwrap();
    }

    #[test]
    fn no_main_link_requires_icarus_and_declares_verilog_link_output() {
        let mut plan = complete_plan();
        plan.scenarios[0].requires = vec![Requirement::Verilog, Requirement::Icarus];
        plan.fixtures.extend([
            Fixture {
                path: "Tb.v".to_owned(),
                source: None,
                sha256: "2".repeat(64),
                role: FixtureRole::BuildInput,
            },
            Fixture {
                path: "mkDemo.v".to_owned(),
                source: None,
                sha256: "3".repeat(64),
                role: FixtureRole::BuildInput,
            },
        ]);
        plan.scenarios[0]
            .fixtures
            .extend(["Tb.v".to_owned(), "mkDemo.v".to_owned()]);
        let provenance = plan.scenarios[0].stages[0].operations[0].provenance.clone();
        plan.scenarios[0].stages[0].operations[0] = OperationRecord::new(
            Action::BscLink {
                backend: SimulationBackend::Icarus,
                mode: BscLinkMode::NoMain,
                objects: vec!["Tb.v".to_owned(), "mkDemo.v".to_owned()],
                top: "Tb".to_owned(),
                args: Vec::new(),
                expected_exit: ExpectedExit::Success,
                simulator: IcarusSimulatorSelector::Default,
                missing_objects: Vec::new(),
            },
            OperationExpectation::Required,
            provenance.clone(),
        );
        assert!(plan.validate().is_ok());
        assert!(plan.scenarios[0].stages[0].operations[0]
            .artifacts
            .outputs
            .contains(&"Tb.bsc-vcomp-out".to_owned()));

        plan.scenarios[0].stages[0].operations[0] = OperationRecord::new(
            Action::BscLink {
                backend: SimulationBackend::Bluesim,
                mode: BscLinkMode::NoMain,
                objects: vec!["Tb.v".to_owned(), "mkDemo.v".to_owned()],
                top: "Tb".to_owned(),
                args: Vec::new(),
                expected_exit: ExpectedExit::Success,
                simulator: IcarusSimulatorSelector::Default,
                missing_objects: Vec::new(),
            },
            OperationExpectation::Required,
            provenance.clone(),
        );
        assert!(plan.validate().is_err());

        plan.scenarios[0].stages[0].operations[0] = OperationRecord::new(
            Action::BscLink {
                backend: SimulationBackend::Bluesim,
                mode: BscLinkMode::NoMain,
                objects: vec!["Tb.v".to_owned(), "mkDemo.v".to_owned()],
                top: "Tb".to_owned(),
                args: Vec::new(),
                expected_exit: ExpectedExit::Failure,
                simulator: IcarusSimulatorSelector::Default,
                missing_objects: Vec::new(),
            },
            OperationExpectation::Required,
            provenance,
        );
        assert!(plan.validate().is_err());
    }

    #[test]
    fn failed_link_declares_log_but_not_executable() {
        let operation = OperationRecord::new(
            Action::BscLink {
                backend: SimulationBackend::Bluesim,
                mode: BscLinkMode::Standard,
                objects: vec!["mkDemo.ba".to_owned()],
                top: "mkDemo".to_owned(),
                args: Vec::new(),
                expected_exit: ExpectedExit::Failure,
                simulator: IcarusSimulatorSelector::Default,
                missing_objects: Vec::new(),
            },
            OperationExpectation::Required,
            Provenance {
                span: span(),
                expansion: Vec::new(),
            },
        );
        assert!(operation
            .artifacts
            .outputs
            .contains(&"mkDemo.bsc-ccomp-out".to_owned()));
        assert!(!operation.artifacts.outputs.contains(&"mkDemo".to_owned()));
        assert!(!operation
            .artifacts
            .outputs
            .contains(&"mkDemo.exe".to_owned()));
    }

    #[test]
    fn rejects_unsafe_modules_link_top_and_internal_namespace_aliases() {
        let mut plan = complete_plan();
        if let Action::BscGenerate { module, .. } =
            &mut plan.scenarios[0].stages[0].operations[0].action
        {
            *module = Some("../outside".to_owned());
        }
        assert!(plan.validate().is_err());

        let mut plan = complete_plan();
        plan.scenarios[0].stages[0].operations[0].action = Action::BscLink {
            backend: SimulationBackend::Bluesim,
            mode: BscLinkMode::Standard,
            objects: vec!["GCD.ba".to_owned()],
            top: "../outside".to_owned(),
            args: Vec::new(),
            expected_exit: ExpectedExit::Success,
            simulator: IcarusSimulatorSelector::Default,
            missing_objects: Vec::new(),
        };
        assert!(plan.validate().is_err());

        let mut plan = complete_plan();
        plan.scenarios[0].stages[0].operations[0].action = Action::BscLink {
            backend: SimulationBackend::Bluesim,
            mode: BscLinkMode::Standard,
            objects: vec!["GCD.ba".to_owned()],
            top: ".BSC-TEST-PLAN".to_owned(),
            args: Vec::new(),
            expected_exit: ExpectedExit::Success,
            simulator: IcarusSimulatorSelector::Default,
            missing_objects: Vec::new(),
        };
        assert!(plan.validate().is_err());

        let mut plan = complete_plan();
        plan.scenarios[0].stages[0].operations[0].action = Action::FsRemove {
            path: ".BSC-TEST-PLAN/assertions/0".to_owned(),
        };
        assert!(plan.validate().is_err());
    }

    #[test]
    fn rejects_windows_path_collisions() {
        let mut plan = complete_plan();
        plan.scenarios[0].stages[0].operations[0].action = Action::AssertGolden {
            actual: "Foo.out".to_owned(),
            expected: "foo.out".to_owned(),
        };
        assert!(plan.validate().is_err());

        let mut plan = complete_plan();
        plan.fixtures.push(Fixture {
            path: "gcd.BSV".to_owned(),
            source: None,
            sha256: "2".repeat(64),
            role: FixtureRole::Source,
        });
        assert!(plan.validate().is_err());
    }

    #[test]
    fn rejects_unsafe_paths_hidden_in_argv() {
        let mut plan = complete_plan();
        if let Action::BscGenerate { args, .. } =
            &mut plan.scenarios[0].stages[0].operations[0].action
        {
            *args = vec!["-bdir".to_owned(), "../../outside".to_owned()];
        }
        assert!(plan.validate().is_err());

        let mut plan = complete_plan();
        if let Action::BscGenerate { args, .. } =
            &mut plan.scenarios[0].stages[0].operations[0].action
        {
            *args = vec!["-simdir=.BSC-TEST-PLAN/assertions".to_owned()];
        }
        assert!(plan.validate().is_err());

        let mut plan = complete_plan();
        if let Action::BscGenerate { args, .. } =
            &mut plan.scenarios[0].stages[0].operations[0].action
        {
            *args = vec!["-bdir".to_owned(), "C:outside".to_owned()];
        }
        assert!(plan.validate().is_err());
    }

    #[test]
    fn accepts_escaped_quotes_in_non_path_argv() {
        let mut plan = complete_plan();
        if let Action::BscGenerate { args, .. } =
            &mut plan.scenarios[0].stages[0].operations[0].action
        {
            *args = vec![r#"-DMESSAGE=\"don't panic\""#.to_owned()];
        }
        plan.validate().unwrap();

        let mut plan = complete_plan();
        if let Action::BscGenerate { args, .. } =
            &mut plan.scenarios[0].stages[0].operations[0].action
        {
            *args = vec![r"-I\outside".to_owned()];
        }
        assert!(plan.validate().is_err());
    }

    #[test]
    fn rejects_case_collisions_across_operations() {
        let mut plan = complete_plan();
        plan.scenarios[0].stages[0].operations = vec![
            OperationRecord::new(
                Action::FsMkdir {
                    path: "Foo".to_owned(),
                },
                OperationExpectation::Required,
                Provenance {
                    span: span(),
                    expansion: Vec::new(),
                },
            ),
            OperationRecord::new(
                Action::AssertExists {
                    path: "foo".to_owned(),
                },
                OperationExpectation::Required,
                Provenance {
                    span: span(),
                    expansion: Vec::new(),
                },
            ),
        ];
        assert!(plan.validate().is_err());

        let mut plan = complete_plan();
        if let Action::BscGenerate { module, .. } =
            &mut plan.scenarios[0].stages[0].operations[0].action
        {
            *module = Some("Foo".to_owned());
        }
        plan.scenarios[0].stages[0]
            .operations
            .push(OperationRecord::new(
                Action::BscLink {
                    backend: SimulationBackend::Bluesim,
                    mode: BscLinkMode::Standard,
                    objects: vec!["foo.ba".to_owned()],
                    top: "mkTop".to_owned(),
                    args: Vec::new(),
                    expected_exit: ExpectedExit::Success,
                    simulator: IcarusSimulatorSelector::Default,
                    missing_objects: Vec::new(),
                },
                OperationExpectation::Required,
                Provenance {
                    span: span(),
                    expansion: Vec::new(),
                },
            ));
        assert!(plan.validate().is_err());
    }

    #[test]
    fn rejects_case_collisions_in_the_plan_index() {
        let entry = |id: &str, path: &str| TestPlanIndexEntry {
            id: id.to_owned(),
            path: path.to_owned(),
            origin: Origin {
                path: "testsuite/example.exp".to_owned(),
                sha256: "0".repeat(64),
            },
            status: PlanStatus::Blocked,
            scenario_count: 0,
            stage_count: 0,
            operation_count: 0,
            diagnostic_count: 1,
        };
        let index = TestPlanIndex {
            schema_version: TEST_PLAN_INDEX_SCHEMA_VERSION,
            plans: vec![
                entry("Example", "Example.test.json"),
                entry("example", "other.test.json"),
            ],
        };
        assert!(index.validate().is_err());

        let index = TestPlanIndex {
            schema_version: TEST_PLAN_INDEX_SCHEMA_VERSION,
            plans: vec![
                entry("one", "Example.test.json"),
                entry("two", "example.test.json"),
            ],
        };
        assert!(index.validate().is_err());
    }

    #[test]
    fn windows_incompatible_paths_require_non_windows() {
        for path in ["dir:with/simulation.out", "NUL.txt", "trailing-dot."] {
            let mut plan = complete_plan();
            let provenance = plan.scenarios[0].stages[0].operations[0].provenance.clone();
            plan.scenarios[0].stages[0].operations[0] = OperationRecord::new(
                Action::FsMkdir {
                    path: path.to_owned(),
                },
                OperationExpectation::Required,
                provenance,
            );
            assert!(plan.validate().is_err(), "{path} must require non_windows");
        }

        let mut plan = complete_plan();
        plan.scenarios[0].requires.push(Requirement::NonWindows);
        let provenance = plan.scenarios[0].stages[0].operations[0].provenance.clone();
        plan.scenarios[0].stages[0].operations[0] = OperationRecord::new(
            Action::FsMkdir {
                path: "dir:with,many;spec#ial=char%acters".to_owned(),
            },
            OperationExpectation::Required,
            provenance,
        );
        plan.validate().unwrap();
    }

    #[test]
    fn compile_does_not_invent_an_unstable_object_artifact() {
        let operation = OperationRecord::new(
            Action::BscCompile {
                source: "Design.bsv".to_owned(),
                working_directory: None,
                mode: BscCompileMode::Frontend,
                module: None,
                args: vec!["-bdir".to_owned(), "build".to_owned()],
                absolute_import_paths: Vec::new(),
                dependency_mode: DependencyMode::Update,
                expected_exit: ExpectedExit::Success,
                unexpected_success_forbidden_regex: None,
                environment: None,
                stdout: "Design.bsv.bsc-out".to_owned(),
            },
            OperationExpectation::Required,
            complete_plan().scenarios[0].stages[0].operations[0]
                .provenance
                .clone(),
        );

        assert_eq!(operation.artifacts.outputs, ["Design.bsv.bsc-out"]);
    }

    #[test]
    fn compile_absolute_import_paths_are_closed_workspace_directories() {
        let compile = |paths: Vec<String>, args: Vec<String>| Action::BscCompile {
            source: "IncludeTest.bsv".to_owned(),
            working_directory: None,
            mode: BscCompileMode::Frontend,
            module: None,
            args,
            absolute_import_paths: paths,
            dependency_mode: DependencyMode::Update,
            expected_exit: ExpectedExit::Success,
            unexpected_success_forbidden_regex: None,
            environment: None,
            stdout: "IncludeTest.bsv.bsc-out".to_owned(),
        };

        validate_action(&compile(vec!["incfiles".to_owned()], Vec::new())).unwrap();
        for paths in [
            vec!["../incfiles".to_owned()],
            vec!["incfiles".to_owned(), "incfiles".to_owned()],
        ] {
            assert!(validate_action(&compile(paths, Vec::new())).is_err());
        }
        assert!(validate_action(&compile(
            vec!["incfiles".to_owned()],
            vec!["-p".to_owned(), "+:other".to_owned()],
        ))
        .is_err());
    }

    #[test]
    fn touch_declares_a_mutating_file_contract() {
        let operation = OperationRecord::new(
            Action::FsTouch {
                path: "Source.bsv".to_owned(),
            },
            OperationExpectation::Required,
            complete_plan().scenarios[0].stages[0].operations[0]
                .provenance
                .clone(),
        );

        assert_eq!(operation.artifacts.inputs, ["Source.bsv"]);
        assert_eq!(operation.artifacts.outputs, ["Source.bsv"]);
    }

    #[test]
    fn typed_workspace_actions_have_closed_schema_and_artifact_contracts() {
        let provenance = complete_plan().scenarios[0].stages[0].operations[0]
            .provenance
            .clone();
        let touch = OperationRecord::new(
            Action::FsTouchCreate {
                path: "Generated.bsv".to_owned(),
                delay_milliseconds: 1000,
            },
            OperationExpectation::Required,
            provenance.clone(),
        );
        assert!(touch.artifacts.inputs.is_empty());
        assert_eq!(touch.artifacts.outputs, ["Generated.bsv"]);
        assert_eq!(
            serde_json::to_value(&touch).unwrap()["op"],
            "fs.touch_create"
        );

        let render = OperationRecord::new(
            Action::M4CurdirRender {
                template: "Source.pre-m4".to_owned(),
                output: "Source.bsv".to_owned(),
            },
            OperationExpectation::Required,
            provenance.clone(),
        );
        assert_eq!(render.artifacts.inputs, ["Source.pre-m4"]);
        assert_eq!(render.artifacts.outputs, ["Source.bsv"]);

        let mut invalid_delay = complete_plan();
        invalid_delay.scenarios[0].stages[0].operations[0] = OperationRecord::new(
            Action::FsTouchCreate {
                path: "Generated.bsv".to_owned(),
                delay_milliseconds: 0,
            },
            OperationExpectation::Required,
            provenance.clone(),
        );
        assert!(invalid_delay.validate().is_err());

        let mut unreadable = complete_plan();
        unreadable.scenarios[0].stages[0].operations[0] = OperationRecord::new(
            Action::FsRemoveUserRead {
                path: "GCD.bsv".to_owned(),
            },
            OperationExpectation::Required,
            provenance,
        );
        assert!(unreadable.validate().is_err());
        unreadable.scenarios[0].stages[0].operations[0]
            .requires
            .push(Requirement::PosixUnreadability);
        unreadable.validate().unwrap();
    }

    #[test]
    fn ensure_directory_absent_declares_an_idempotent_remove_contract() {
        let operation = OperationRecord::new(
            Action::FsEnsureDirectoryAbsent {
                path: "work".to_owned(),
            },
            OperationExpectation::Required,
            complete_plan().scenarios[0].stages[0].operations[0]
                .provenance
                .clone(),
        );

        assert!(operation.artifacts.inputs.is_empty());
        assert!(operation.artifacts.outputs.is_empty());
        assert!(operation.artifacts.directories.is_empty());
        assert_eq!(operation.artifacts.removes, ["work"]);
        let rendered = serde_json::to_value(&operation).unwrap();
        assert_eq!(rendered["op"], "fs.ensure_dir_absent");
    }

    #[test]
    fn mkdir_declares_a_directory_contract() {
        let operation = OperationRecord::new(
            Action::FsMkdir {
                path: "work".to_owned(),
            },
            OperationExpectation::Required,
            complete_plan().scenarios[0].stages[0].operations[0]
                .provenance
                .clone(),
        );

        assert_eq!(operation.artifacts.directories, ["work"]);
        assert!(operation.artifacts.outputs.is_empty());
    }

    #[test]
    fn windows_incompatible_fixtures_require_all_scenarios_to_be_non_windows() {
        let mut plan = complete_plan();
        plan.fixtures[0].path = "AUX.bsv".to_owned();
        plan.scenarios[0].fixtures[0] = "AUX.bsv".to_owned();
        let provenance = plan.scenarios[0].stages[0].operations[0].provenance.clone();
        plan.scenarios[0].stages[0].operations[0] = OperationRecord::new(
            Action::BscGenerate {
                source: "AUX.bsv".to_owned(),
                mode: SimulationGenerationMode::Bluesim,
                module: Some("mkGCD".to_owned()),
                args: Vec::new(),
            },
            OperationExpectation::Required,
            provenance,
        );
        assert!(plan.validate().is_err());

        plan.scenarios[0].requires.push(Requirement::NonWindows);
        plan.validate().unwrap();
    }

    #[test]
    fn rejects_unsafe_simulation_output_paths() {
        for action in [
            Action::SimulationRun {
                backend: SimulationBackend::Bluesim,
                executable: "mkGCD".to_owned(),
                args: Vec::new(),
                stdout: "../simulation.out".to_owned(),
                expected_exits: ExpectedExitSet::default(),
                vcd: None,
            },
            Action::SimulationRun {
                backend: SimulationBackend::Icarus,
                executable: "mkGCD".to_owned(),
                args: Vec::new(),
                stdout: "simulation.out".to_owned(),
                expected_exits: ExpectedExitSet::default(),
                vcd: Some(".bsc-test-plan/assertions/simulation.vcd".to_owned()),
            },
        ] {
            let mut plan = complete_plan();
            plan.scenarios[0].stages[0].operations[0].action = action;
            assert!(plan.validate().is_err());
        }
    }

    #[test]
    fn typed_text_filter_and_link_actions_round_trip_with_exact_artifact_contracts() {
        let normalize = Action::TextNormalize {
            source: "raw.out".to_owned(),
            destination: "sorted.out".to_owned(),
            transform: TextNormalization::SortNumericField1ThenField2,
        };
        assert_eq!(
            ArtifactContract::for_action(&normalize),
            ArtifactContract {
                inputs: vec!["raw.out".to_owned()],
                outputs: vec!["sorted.out".to_owned()],
                output_alternatives: Vec::new(),
                directories: Vec::new(),
                removes: Vec::new(),
            }
        );

        let filter = Action::VerilogFilter {
            path: "mkTop.v".to_owned(),
            profiles: vec![
                VerilogFilterProfile::RenameFire,
                VerilogFilterProfile::RenameFire,
                VerilogFilterProfile::ClockToClock,
            ],
            expected_exit: ExpectedExit::Success,
        };
        let filter_artifacts = ArtifactContract::for_action(&filter);
        assert_eq!(
            filter_artifacts.inputs,
            ["mkTop.v", "renamefire.pl", "simple.sed"]
        );
        assert_eq!(filter_artifacts.outputs, ["mkTop.v"]);

        for action in [&normalize, &filter] {
            let serialized = serde_json::to_value(action).unwrap();
            assert_eq!(
                serde_json::from_value::<Action>(serialized).unwrap(),
                *action
            );
            validate_action(action).unwrap();
        }

        let link = |simulator, expected_exit| Action::BscLink {
            backend: SimulationBackend::Icarus,
            mode: BscLinkMode::Standard,
            objects: vec!["mkTop".to_owned()],
            top: "sysTop".to_owned(),
            args: Vec::new(),
            expected_exit,
            simulator,
            missing_objects: Vec::new(),
        };
        for (selector, expected_exit, expected_outputs) in [
            (
                IcarusSimulatorSelector::Default,
                ExpectedExit::Success,
                vec!["sysTop.bsc-vcomp-out", "sysTop.vexe"],
            ),
            (
                IcarusSimulatorSelector::BluespecDirInstalledBuilder,
                ExpectedExit::Success,
                vec!["sysTop.bsc-vcomp-out", "sysTop.vexe"],
            ),
            (
                IcarusSimulatorSelector::PosixEchoProbe,
                ExpectedExit::Success,
                vec!["sysTop.bsc-vcomp-out"],
            ),
            (
                IcarusSimulatorSelector::LiteralBogus,
                ExpectedExit::Failure,
                vec!["sysTop.bsc-vcomp-out"],
            ),
            (
                IcarusSimulatorSelector::BluespecDirBogus,
                ExpectedExit::Failure,
                vec!["sysTop.bsc-vcomp-out"],
            ),
        ] {
            let action = link(selector, expected_exit);
            validate_action(&action).unwrap();
            assert_eq!(
                ArtifactContract::for_action(&action).outputs,
                expected_outputs
            );
            let serialized = serde_json::to_value(&action).unwrap();
            assert_eq!(
                serde_json::from_value::<Action>(serialized).unwrap(),
                action
            );
        }
    }

    #[test]
    fn typed_text_filter_and_simulator_validation_rejects_near_matches() {
        assert!(validate_action(&Action::TextNormalize {
            source: "Result.out".to_owned(),
            destination: "result.OUT".to_owned(),
            transform: TextNormalization::BluesimTaskProjection,
        })
        .is_err());

        let filter = |profiles, expected_exit| Action::VerilogFilter {
            path: "mkTop.v".to_owned(),
            profiles,
            expected_exit,
        };
        validate_action(&filter(
            vec![
                VerilogFilterProfile::RenameFire,
                VerilogFilterProfile::MissingSed,
            ],
            ExpectedExit::Failure,
        ))
        .unwrap();
        for action in [
            filter(Vec::new(), ExpectedExit::Success),
            filter(
                vec![
                    VerilogFilterProfile::MissingSed,
                    VerilogFilterProfile::RenameFire,
                ],
                ExpectedExit::Failure,
            ),
            filter(
                vec![VerilogFilterProfile::MissingSed],
                ExpectedExit::Success,
            ),
            filter(
                vec![VerilogFilterProfile::RenameFire],
                ExpectedExit::Failure,
            ),
        ] {
            assert!(validate_action(&action).is_err());
        }

        let link = |backend, mode, simulator, expected_exit| Action::BscLink {
            backend,
            mode,
            objects: vec!["mkTop".to_owned()],
            top: "sysTop".to_owned(),
            args: Vec::new(),
            expected_exit,
            simulator,
            missing_objects: Vec::new(),
        };
        for action in [
            link(
                SimulationBackend::Icarus,
                BscLinkMode::Standard,
                IcarusSimulatorSelector::LiteralBogus,
                ExpectedExit::Success,
            ),
            link(
                SimulationBackend::Bluesim,
                BscLinkMode::Standard,
                IcarusSimulatorSelector::BluespecDirInstalledBuilder,
                ExpectedExit::Success,
            ),
            link(
                SimulationBackend::Icarus,
                BscLinkMode::NoMain,
                IcarusSimulatorSelector::PosixEchoProbe,
                ExpectedExit::Success,
            ),
        ] {
            assert!(validate_action(&action).is_err());
        }

        let mut serialized = serde_json::to_value(link(
            SimulationBackend::Icarus,
            BscLinkMode::Standard,
            IcarusSimulatorSelector::Default,
            ExpectedExit::Success,
        ))
        .unwrap();
        serialized["simulator"] = serde_json::json!("C:/host/tool.exe");
        assert!(serde_json::from_value::<Action>(serialized).is_err());
    }

    #[test]
    fn schema_contains_the_closed_operation_vocabulary() {
        let schema = render_schema().unwrap();
        for operation in [
            "bsc.compile",
            "bsc.generate",
            "bsc.link",
            "golden.render",
            "text.normalize",
            "verilog.filter",
            "simulation.run",
            "vcd.check",
            "assert.regex",
            "assert.vcd",
            "assert.vcd_valid",
        ] {
            assert!(schema.contains(operation));
        }
        assert!(schema.contains("shared_elaboration"));
        assert!(schema.contains("bluespec_dir_installed_builder"));
        assert!(schema.contains("system_verilog_task_diagnostics"));
        assert!(schema.contains("wf_to_w_f"));
        assert!(!schema.contains("bluesim.run"));
        assert!(!schema.contains("\"shell\""));
        assert!(!schema.contains("\"eval\""));
    }
}
