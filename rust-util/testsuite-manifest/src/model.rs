use serde::{Deserialize, Serialize};

pub const MANIFEST_SCHEMA_VERSION: u32 = 4;

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
    pub bluesim_workflows: Vec<BluesimWorkflow>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub workflow_actions: Vec<WorkflowAction>,
    pub unsupported: Vec<UnsupportedConstruct>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Contract {
    Compile(CompileContract),
    Simulation(SimulationContract),
    ExternalSet(ExternalSetContract),
}

impl Contract {
    pub fn effective_count(&self) -> usize {
        match self {
            Self::Compile(_) | Self::Simulation(_) => 1,
            Self::ExternalSet(contract) => contract.cases.len(),
        }
    }

    pub fn guard(&self) -> &Guard {
        match self {
            Self::Compile(contract) => &contract.guard,
            Self::Simulation(contract) => &contract.guard,
            Self::ExternalSet(contract) => &contract.guard,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompileContract {
    pub source: String,
    pub helper: String,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulationContract {
    pub source: String,
    pub helper: String,
    pub backend: SimulationBackend,
    pub generation: GenerationStrategy,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BluesimWorkflow {
    pub top: String,
    pub generations: Vec<CompileObjectAction>,
    pub link: LinkObjectsAction,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkflowAction {
    CompileObject(CompileObjectAction),
    LinkObjects(LinkObjectsAction),
    RunBluesim(RunBluesimAction),
    TransferArtifact(ArtifactTransferAction),
}

impl WorkflowAction {
    pub fn helper_name(&self) -> &'static str {
        match self {
            Self::CompileObject(_) => "compile_object_pass",
            Self::LinkObjects(_) => "link_objects_pass",
            Self::RunBluesim(_) => "sim_output",
            Self::TransferArtifact(action) => match action.operation {
                ArtifactTransferOperation::Copy => "copy",
                ArtifactTransferOperation::Move => "move",
            },
        }
    }

    pub fn guard(&self) -> &Guard {
        match self {
            Self::CompileObject(action) => &action.guard,
            Self::LinkObjects(action) => &action.guard,
            Self::RunBluesim(action) => &action.guard,
            Self::TransferArtifact(action) => &action.guard,
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
pub struct LinkObjectsAction {
    pub objects: String,
    pub top: String,
    pub options: String,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunBluesimAction {
    pub executable: String,
    pub options: String,
    pub stdout: String,
    pub guard: Guard,
    pub span: SourceSpan,
    pub expansion: Vec<SourceSpan>,
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
    pub bluesim_workflows: usize,
    pub bluesim_workflow_contracts: usize,
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
                    Contract::Compile(_) => summary.compile_contracts += 1,
                    Contract::Simulation(_) => summary.simulation_contracts += 1,
                    Contract::ExternalSet(contract) => {
                        summary.external_contract_sets += 1;
                        summary.external_contracts += contract.cases.len();
                    }
                }
                summary.unresolved_contracts +=
                    usize::from(!contract.guard().is_resolved()) * contract.effective_count();
            }
            summary.assertions += script.assertions.len();
            summary.comparisons += script.comparisons.len();
            summary.bluesim_workflows += script.bluesim_workflows.len();
            summary.bluesim_workflow_contracts += script
                .bluesim_workflows
                .iter()
                .map(BluesimWorkflow::effective_count)
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
