//! Origins are the sole `.exp` scripts in the fixture directories declared by
//! each case (`gh309`, `gh435`, three `bugID*`, three message directories, and
//! `bsc.syntax/bsv05/moduletype`).

use super::CompileCase;

pub(super) const GH435: CompileCase = compile_pass_case!(
    "bsc.bugs/github/gh435::Top.bs",
    "testsuite/bsc.bugs/github/gh435",
    "Top.bs"
);
pub(super) const GH309: CompileCase = compile_pass_case!(
    "bsc.bugs/github/gh309::ICE.bs",
    "testsuite/bsc.bugs/github/gh309",
    "ICE.bs"
);
pub(super) const BUG_ID_313: CompileCase = compile_fail_golden_case!(
    "bsc.interra/bugs/bugID313::bug.bsv",
    "testsuite/bsc.interra/bugs/bugID313",
    "bug.bsv",
    "bug.bsv.bsc-out.expected"
);
pub(super) const BUG_ID_149: CompileCase = compile_fail_golden_case!(
    "bsc.interra/bugs/bugID149::EDupField2.bs",
    "testsuite/bsc.interra/bugs/bugID149",
    "EDupField2.bs",
    "EDupField2.bs.bsc-out.expected"
);
pub(super) const BUG_ID_169: CompileCase = compile_fail_golden_case!(
    "bsc.interra/bugs/bugID169::Test.bs",
    "testsuite/bsc.interra/bugs/bugID169",
    "Test.bs",
    "Test.bs.bsc-out.expected"
);
pub(super) const E_MISSING_NL: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EMissingNL::EMissingNL.bs",
    "testsuite/bsc.interra/messages/EMissingNL",
    "EMissingNL.bs",
    "EMissingNL.bs.bsc-out.expected"
);
pub(super) const E_UNBOUND_CL_CON: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EUnboundClCon::EUnboundClCon.bs",
    "testsuite/bsc.interra/messages/EUnboundClCon",
    "EUnboundClCon.bs",
    "EUnboundClCon.bs.bsc-out.expected"
);
pub(super) const E_FIELD_AMB: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EFieldAmb::EFieldAmb.bs",
    "testsuite/bsc.interra/messages/EFieldAmb",
    "EFieldAmb.bs",
    "EFieldAmb.bs.bsc-out.expected"
);
pub(super) const MODULE_TYPE_GOOD: CompileCase = compile_pass_case!(
    "bsc.syntax/bsv05/moduletype::ModuleTypeInFunction.bsv",
    "testsuite/bsc.syntax/bsv05/moduletype",
    "ModuleTypeInFunction.bsv"
);
pub(super) const MODULE_TYPE_MISSING_ARG: CompileCase = compile_fail_error_case!(
    "bsc.syntax/bsv05/moduletype::ModuleTypeInFunction_MissingArg.bsv",
    "testsuite/bsc.syntax/bsv05/moduletype",
    "ModuleTypeInFunction_MissingArg.bsv",
    "T0025"
);
