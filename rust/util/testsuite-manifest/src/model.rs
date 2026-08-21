use bsc_test_plan::{
    BluetclInstalledScript, BluetclMakedependCommand, BluetclPackage, ExpectedExit,
    OperationExpectation,
};
use serde::{Deserialize, Serialize};

pub const MANIFEST_SCHEMA_VERSION: u32 = 36;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestsuiteManifest {
    pub schema_version: u32,
    pub scripts: Vec<ScriptManifest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptManifest {
    pub origin: String,
    pub source_sha256: String,
    pub contracts: Vec<Contract>,
    pub assertions: Vec<AssertionContract>,
    pub comparisons: Vec<ComparisonContract>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bluesim_sequences: Vec<BluesimSequence>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bluesim_workflows: Vec<BluesimWorkflow>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub systemc_workflows: Vec<SystemcWorkflow>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub workflow_actions: Vec<WorkflowAction>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub make_test_data_actions: Vec<MakeTestDataAction>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bsc_options_overlays: Vec<BscOptionsOverlay>,
    pub unsupported: Vec<UnsupportedConstruct>,
}

/// A statically delimited `BSC_OPTIONS` append scope recovered without evaluating Tcl.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MakeTestDataAction {
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BscOptionsOverlay {
    pub append: String,
    pub start: SourceSpan,
    pub end: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Contract {
    Compile(CompileContract),
    NoSourceCompile(NoSourceCompileContract),
    BasicOptions(BasicOptionsContract),
    Ovl(OvlContract),
    RenderGolden(RenderGoldenContract),
    Simulation(SimulationContract),
    ExternalSet(ExternalSetContract),
}

impl Contract {
    pub fn effective_count(&self) -> usize {
        match self {
            Self::Compile(_)
            | Self::NoSourceCompile(_)
            | Self::BasicOptions(_)
            | Self::Ovl(_)
            | Self::RenderGolden(_)
            | Self::Simulation(_) => 1,
            Self::ExternalSet(contract) => contract.cases.len(),
        }
    }

    pub fn guard(&self) -> &Guard {
        match self {
            Self::Compile(contract) => &contract.guard,
            Self::NoSourceCompile(contract) => &contract.guard,
            Self::BasicOptions(contract) => &contract.guard,
            Self::Ovl(contract) => &contract.guard,
            Self::RenderGolden(contract) => &contract.guard,
            Self::Simulation(contract) => &contract.guard,
            Self::ExternalSet(contract) => &contract.guard,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompileContract {
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,
    pub helper: String,
    pub arguments: Vec<String>,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoSourceCompileContract {
    pub name: String,
    pub options: String,
    pub diagnostic: String,
    pub count: String,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BasicOptionsContract {
    pub options: String,
    pub output: String,
    pub expected: String,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OvlContract {
    pub case_dir: String,
    pub top: String,
    pub library: String,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderGoldenContract {
    pub template: String,
    pub output: String,
    pub macro_value: GoldenMacroValue,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoldenMacroValue {
    BluespecDir,
    WorkDir,
    FifoWarningLocations,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulationContract {
    pub source: String,
    pub helper: String,
    pub arguments: Vec<String>,
    pub backend: SimulationBackend,
    pub generation: GenerationStrategy,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BluesimSequence {
    pub contracts: Vec<BluesimSequenceContract>,
}

impl BluesimSequence {
    pub fn effective_count(&self) -> usize {
        self.contracts.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BluesimSequenceContract {
    pub operations: Vec<WorkflowOperation>,
}

impl BluesimSequenceContract {
    pub fn actions(&self) -> impl Iterator<Item = &WorkflowAction> {
        self.operations
            .iter()
            .filter_map(|operation| match operation {
                WorkflowOperation::Action(action) => Some(action),
                WorkflowOperation::Assertion(_) => None,
            })
    }

    pub fn assertions(&self) -> impl Iterator<Item = &AssertionContract> {
        self.operations
            .iter()
            .filter_map(|operation| match operation {
                WorkflowOperation::Assertion(assertion) => Some(assertion),
                WorkflowOperation::Action(_) => None,
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum WorkflowOperation {
    Action(WorkflowAction),
    Assertion(AssertionContract),
}

impl WorkflowOperation {
    pub fn guard(&self) -> &Guard {
        match self {
            Self::Action(action) => action.guard(),
            Self::Assertion(assertion) => &assertion.guard,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BluesimWorkflow {
    pub top: String,
    pub generations: Vec<CompileObjectAction>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pre_link_transfers: Vec<ArtifactTransferAction>,
    pub link: LinkObjectsAction,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub link_transfers: Vec<ArtifactTransferAction>,
    pub runs: Vec<BluesimRun>,
}

impl BluesimWorkflow {
    pub fn effective_count(&self) -> usize {
        self.runs.len().max(1)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BluesimRun {
    pub action: RunBluesimAction,
    pub transfers: Vec<ArtifactTransferAction>,
}

/// A closed SystemC workflow: BSV generation, BSC SystemC-model link, optional
/// fixed C++ link, and optional SystemC executable run.  The action order is
/// source order and is intentionally distinct from a Bluesim workflow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemcWorkflow {
    pub operations: Vec<WorkflowAction>,
}

impl SystemcWorkflow {
    pub fn effective_count(&self) -> usize {
        1
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkflowAction {
    CompileObject(CompileObjectAction),
    BuildCObject(CObjectBuildAction),
    LinkObjects(LinkObjectsAction),
    LinkVerilog(LinkVerilogAction),
    RunBluesim(RunBluesimAction),
    RunVerilog(RunVerilogAction),
    ShowRules(ShowRulesAction),
    LinkSystemc(SystemcLinkAction),
    BuildSystemc(SystemcBuildAction),
    RunSystemc(RunSystemcAction),
    BluetclRun(BluetclRunAction),
    Bsc2Bsv(Bsc2BsvAction),
    BscParsePretty(BscParsePrettyAction),
    TransferArtifact(ArtifactTransferAction),
    EraseArtifact(EraseArtifactAction),
    EnsureDirectoryAbsent(EnsureDirectoryAbsentAction),
    CreateDirectory(CreateDirectoryAction),
    TouchArtifact(TouchArtifactAction),
    TouchCreateArtifact(TouchCreateArtifactAction),
    RemoveUserRead(RemoveUserReadAction),
    RewriteDarwinCppIncludePath(RewriteDarwinCppIncludePathAction),
    RenderGolden(RenderGoldenAction),
    RenderM4Curdir(RenderM4CurdirAction),
    TextNormalize(TextNormalizeAction),
    VerilogFilter(VerilogFilterAction),
    Delay(DelayAction),
    DumpIntermediate(DumpIntermediateAction),
}

impl WorkflowAction {
    pub fn helper_name(&self) -> &'static str {
        match self {
            Self::CompileObject(_) => "compile_object_pass",
            Self::BuildCObject(_) => "make_pass",
            Self::LinkObjects(action) if action.error_diagnostic.is_some() => {
                "link_objects_fail_error"
            }
            Self::LinkObjects(action)
                if matches!(action.expectation, OperationExpectation::Xfail { .. }) =>
            {
                "link_objects_pass_bug"
            }
            Self::LinkObjects(action) if action.expected_exit == ExpectedExit::Failure => {
                "link_objects_fail"
            }
            Self::LinkObjects(_) => "link_objects_pass",
            Self::LinkVerilog(action)
                if matches!(action.expectation, OperationExpectation::Xfail { .. }) =>
            {
                "link_verilog_pass_bug"
            }
            Self::LinkVerilog(action) if action.no_main => "link_verilog_no_main_pass",
            Self::LinkVerilog(action) if action.expected_exit == ExpectedExit::Failure => {
                "link_verilog_fail"
            }
            Self::LinkVerilog(_) => "link_verilog_pass",
            Self::RunBluesim(action) if action.expected_exits.is_empty() => "sim_output",
            Self::RunBluesim(_) => "sim_output_status",
            Self::RunVerilog(action) if action.vcd => "sim_verilog_vcd",
            Self::RunVerilog(action) if action.expected_exits.is_empty() => "sim_verilog",
            Self::RunVerilog(_) => "sim_verilog_status",
            Self::ShowRules(_) => "showrules",
            Self::LinkSystemc(action) if action.error_diagnostic.is_some() => {
                "create_systemc_objects_fail_error"
            }
            Self::LinkSystemc(_) => "create_systemc_objects_pass",
            Self::BuildSystemc(_) => "build_systemc_executable_pass",
            Self::RunSystemc(_) => "run_systemc_executable",
            Self::BluetclRun(_) => "bluetcl_run",
            Self::Bsc2Bsv(_) => "run_bsc2bsv",
            Self::BscParsePretty(action) => {
                if matches!(action.expectation, OperationExpectation::Xfail { .. }) {
                    "compile_ppp_pass_bug"
                } else {
                    "compile_ppp_pass"
                }
            }
            Self::TransferArtifact(action) => match action.operation {
                ArtifactTransferOperation::Copy => "copy",
                ArtifactTransferOperation::Move => "move",
            },
            Self::EraseArtifact(_) => "erase",
            Self::EnsureDirectoryAbsent(_) => "nukedir",
            Self::CreateDirectory(_) => "mkdir",
            Self::TouchArtifact(_) | Self::TouchCreateArtifact(_) => "touch",
            Self::RemoveUserRead(_) => "chmod_u_minus_r",
            Self::RewriteDarwinCppIncludePath(_) => "sed_darwin_cpp_include_path",
            Self::RenderGolden(_) => "golden_render",
            Self::RenderM4Curdir(_) => "m4_curdir",
            Self::TextNormalize(_) => "text_normalize",
            Self::VerilogFilter(_) => "verilog_filter",
            Self::Delay(_) => "delay",
            Self::DumpIntermediate(action) => match action.view {
                IntermediateDumpView::Bi => "dumpbi",
                IntermediateDumpView::Bo => "dumpbo",
            },
        }
    }

    pub fn guard(&self) -> &Guard {
        match self {
            Self::CompileObject(action) => &action.guard,
            Self::BuildCObject(action) => &action.guard,
            Self::LinkObjects(action) => &action.guard,
            Self::LinkVerilog(action) => &action.guard,
            Self::RunBluesim(action) => &action.guard,
            Self::RunVerilog(action) => &action.guard,
            Self::ShowRules(action) => &action.guard,
            Self::LinkSystemc(action) => &action.guard,
            Self::BuildSystemc(action) => &action.guard,
            Self::RunSystemc(action) => &action.guard,
            Self::BluetclRun(action) => &action.guard,
            Self::Bsc2Bsv(action) => &action.guard,
            Self::BscParsePretty(action) => &action.guard,
            Self::TransferArtifact(action) => &action.guard,
            Self::EraseArtifact(action) => &action.guard,
            Self::EnsureDirectoryAbsent(action) => &action.guard,
            Self::CreateDirectory(action) => &action.guard,
            Self::TouchArtifact(action) => &action.guard,
            Self::TouchCreateArtifact(action) => &action.guard,
            Self::RemoveUserRead(action) => &action.guard,
            Self::RewriteDarwinCppIncludePath(action) => &action.guard,
            Self::RenderGolden(action) => &action.guard,
            Self::RenderM4Curdir(action) => &action.guard,
            Self::TextNormalize(action) => &action.guard,
            Self::VerilogFilter(action) => &action.guard,
            Self::Delay(action) => &action.guard,
            Self::DumpIntermediate(action) => &action.guard,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompileObjectAction {
    pub source: String,
    pub module: Option<String>,
    pub options: String,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CObjectBuildAction {
    pub source: String,
    pub makefile: String,
    pub output: String,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkObjectsAction {
    pub objects: String,
    pub top: String,
    pub options: String,
    #[serde(default)]
    pub expected_exit: ExpectedExit,
    #[serde(default)]
    pub expectation: OperationExpectation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_diagnostic: Option<LinkErrorDiagnostic>,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkErrorDiagnostic {
    pub code: String,
    pub count: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunBluesimAction {
    pub executable: String,
    pub options: String,
    pub stdout: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expected_exits: Vec<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aarch64_expected_exits: Option<Vec<i32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub windows_expected_exits: Option<Vec<i32>>,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkVerilogAction {
    pub objects: String,
    pub top: String,
    pub options: String,
    #[serde(default)]
    pub no_main: bool,
    #[serde(default)]
    pub expected_exit: ExpectedExit,
    #[serde(default)]
    pub simulator: bsc_test_plan::IcarusSimulatorSelector,
    #[serde(default)]
    pub expectation: OperationExpectation,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunVerilogAction {
    pub executable: String,
    pub options: String,
    pub stdout: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expected_exits: Vec<i32>,
    pub vcd: bool,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShowRulesAction {
    pub top: String,
    pub input: String,
    pub output: String,
    pub stdout: String,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemcLinkAction {
    pub objects: String,
    pub top: String,
    pub options: String,
    pub expected_exit: ExpectedExit,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_diagnostic: Option<LinkErrorDiagnostic>,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemcBuildAction {
    pub executable: String,
    pub sources: String,
    pub top_modules: String,
    pub other_modules: String,
    pub options: String,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunSystemcAction {
    pub executable: String,
    pub options: String,
    pub expected: String,
    pub sort_output: bool,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BluetclRunAction {
    pub invocation: BluetclInvocation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,
    pub artifact_inputs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifact_outputs: Vec<String>,
    #[serde(default)]
    pub expected_exit: ExpectedExit,
    pub stdout: String,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bsc2BsvAction {
    pub source: String,
    pub stdout: String,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BscParsePrettyAction {
    pub source: String,
    pub options: String,
    pub pretty_output: String,
    pub expectation: OperationExpectation,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BluetclSyntax {
    Bsv,
    Bh,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactTransferAction {
    pub operation: ArtifactTransferOperation,
    pub source: String,
    pub destination: String,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EraseArtifactAction {
    pub path: String,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnsureDirectoryAbsentAction {
    pub path: String,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateDirectoryAction {
    pub path: String,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TouchArtifactAction {
    pub path: String,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TouchCreateArtifactAction {
    pub path: String,
    pub delay_milliseconds: u64,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveUserReadAction {
    pub path: String,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RewriteDarwinCppIncludePathAction {
    pub source: String,
    pub destination: String,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderM4CurdirAction {
    pub template: String,
    pub output: String,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderGoldenAction {
    pub template: String,
    pub output: String,
    pub macro_value: GoldenMacroValue,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextNormalizeAction {
    pub source: String,
    pub destination: String,
    pub transform: bsc_test_plan::TextNormalization,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerilogFilterAction {
    pub path: String,
    pub profiles: Vec<bsc_test_plan::VerilogFilterProfile>,
    pub expected_exit: ExpectedExit,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelayAction {
    pub milliseconds: u64,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DumpIntermediateAction {
    pub input: String,
    pub output: String,
    pub view: IntermediateDumpView,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntermediateDumpView {
    Bi,
    Bo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactTransferOperation {
    Copy,
    Move,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalSetContract {
    pub external_kind: ExternalContractKind,
    pub cases: Vec<String>,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalContractKind {
    SchedulerSat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimulationBackend {
    Bluesim,
    Icarus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationStrategy {
    Shared,
    Bluesim,
    Icarus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Bluesim,
    Verilog,
    SystemC,
    ShowRules,
    InternalChecks,
    Darwin,
    BluetclPackage(BluetclPackage),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Guard {
    Always,
    Capability { capability: Capability },
    All { guards: Vec<Guard> },
    Not { guard: Box<Guard> },
    UnsupportedExpression { source: String, span: SourceSpan },
}

impl Guard {
    pub fn is_resolved(&self) -> bool {
        match self {
            Self::Always | Self::Capability { .. } => true,
            Self::All { guards } => guards.iter().all(Self::is_resolved),
            Self::Not { guard } => guard.is_resolved(),
            Self::UnsupportedExpression { .. } => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssertionContract {
    pub helper: String,
    pub arguments: Vec<String>,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComparisonContract {
    pub helper: String,
    pub arguments: Vec<String>,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsupportedConstruct {
    pub command: Option<String>,
    pub reason: UnsupportedReason,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnsupportedReason {
    DynamicAssignment,
    DynamicArguments,
    UnsupportedCommand,
    UnsupportedControlFlow,
    UnsupportedSyntax,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSpan {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ManifestSummary {
    pub scripts: usize,
    pub compile_contracts: usize,
    pub simulation_contracts: usize,
    pub external_contract_sets: usize,
    pub external_contracts: usize,
    pub unresolved_contracts: usize,
    pub assertions: usize,
    pub comparisons: usize,
    pub bluesim_sequences: usize,
    pub bluesim_sequence_contracts: usize,
    pub bluesim_workflows: usize,
    pub bluesim_workflow_contracts: usize,
    pub systemc_workflows: usize,
    pub systemc_workflow_contracts: usize,
    pub workflow_actions: usize,
    pub scripts_with_workflow_actions: usize,
    pub unsupported_constructs: usize,
    pub scripts_with_unsupported: usize,
}

impl TestsuiteManifest {
    pub fn summary(&self) -> ManifestSummary {
        let mut summary = ManifestSummary {
            scripts: self.scripts.len(),
            ..ManifestSummary::default()
        };
        for script in &self.scripts {
            for contract in &script.contracts {
                match contract {
                    Contract::Compile(_) | Contract::NoSourceCompile(_) => {
                        summary.compile_contracts += 1
                    }
                    Contract::BasicOptions(_) | Contract::Ovl(_) | Contract::RenderGolden(_) => {}
                    Contract::Simulation(_) => summary.simulation_contracts += 1,
                    Contract::ExternalSet(contract) => {
                        summary.external_contract_sets += 1;
                        summary.external_contracts += contract.cases.len();
                    }
                }
                summary.unresolved_contracts +=
                    usize::from(!contract.guard().is_resolved()) * contract.effective_count();
            }
            summary.assertions += script.assertions.len()
                + script
                    .bluesim_sequences
                    .iter()
                    .flat_map(|sequence| &sequence.contracts)
                    .map(|contract| contract.assertions().count())
                    .sum::<usize>();
            summary.comparisons += script.comparisons.len();
            summary.bluesim_sequences += script.bluesim_sequences.len();
            summary.bluesim_sequence_contracts += script
                .bluesim_sequences
                .iter()
                .map(BluesimSequence::effective_count)
                .sum::<usize>();
            summary.bluesim_workflows += script.bluesim_workflows.len();
            summary.bluesim_workflow_contracts += script
                .bluesim_workflows
                .iter()
                .map(BluesimWorkflow::effective_count)
                .sum::<usize>();
            summary.systemc_workflows += script.systemc_workflows.len();
            summary.systemc_workflow_contracts += script
                .systemc_workflows
                .iter()
                .map(SystemcWorkflow::effective_count)
                .sum::<usize>();
            summary.workflow_actions += script.workflow_actions.len();
            summary.scripts_with_workflow_actions +=
                usize::from(!script.workflow_actions.is_empty());
            summary.unsupported_constructs += script.unsupported.len();
            summary.scripts_with_unsupported += usize::from(!script.unsupported.is_empty());
        }
        summary
    }
}
