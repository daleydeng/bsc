//! Origins:
//! - `testsuite/bsc.syntax/bsv05/interface/interface.exp`
//! - `testsuite/bsc.typechecker/assignment/assignment.exp`
//! - `testsuite/bsc.interra/messages/EBadIfcType/EBadIfcType.exp`
//! - `testsuite/bsc.preprocessor/ifdef/ifdef.exp`
//! - `testsuite/bsc.driver/symtab/symtab.exp`

use crate::upstream::{CompileCase, CompileExpectation, CompileMode, DiagnosticKind, Requirement};

macro_rules! frontend_case {
    ($name:literal, $fixture_dir:literal, $source:literal, $fixtures:expr, $expectation:expr, $options:expr) => {
        CompileCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: $fixtures,
            assertions: &[],
            expectation: $expectation,
            golden: None,
            options: $options,
            nodeps: false,
            mode: CompileMode::Frontend,
            requirement: Requirement::Always,
        }
    };
}

macro_rules! frontend_pass {
    ($name:literal, $fixture_dir:literal, $source:literal) => {
        frontend_case!(
            $name,
            $fixture_dir,
            $source,
            &[$source],
            CompileExpectation::Pass,
            &[]
        )
    };
    ($name:literal, $fixture_dir:literal, $source:literal, $fixtures:expr) => {
        frontend_case!(
            $name,
            $fixture_dir,
            $source,
            $fixtures,
            CompileExpectation::Pass,
            &[]
        )
    };
}

macro_rules! frontend_fail_with_options {
    ($name:literal, $fixture_dir:literal, $source:literal, $options:expr) => {
        frontend_case!(
            $name,
            $fixture_dir,
            $source,
            &[$source],
            CompileExpectation::Fail,
            $options
        )
    };
}

macro_rules! frontend_fail_error_with_options {
    ($name:literal, $fixture_dir:literal, $source:literal, $tag:literal, $options:expr) => {
        frontend_case!(
            $name,
            $fixture_dir,
            $source,
            &[$source],
            CompileExpectation::FailWithDiagnostic {
                kind: DiagnosticKind::Error,
                tag: $tag,
                count: 1,
            },
            $options
        )
    };
}

macro_rules! verilog_fail_error {
    ($name:literal, $fixture_dir:literal, $source:literal, $tag:literal) => {
        CompileCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: &[$source],
            assertions: &[],
            expectation: CompileExpectation::FailWithDiagnostic {
                kind: DiagnosticKind::Error,
                tag: $tag,
                count: 1,
            },
            golden: None,
            options: &[],
            nodeps: false,
            mode: CompileMode::Verilog { module: None },
            requirement: Requirement::VerilogEnabled,
        }
    };
}

pub(super) const CASES: &[CompileCase] = &[
    // testsuite/bsc.syntax/bsv05/interface/interface.exp (28)
    frontend_pass!(
        "bsc.syntax/bsv05/interface::ModuleInterface_LocalStmt_DeclAssign.bsv",
        "testsuite/bsc.syntax/bsv05/interface",
        "ModuleInterface_LocalStmt_DeclAssign.bsv"
    ),
    frontend_pass!(
        "bsc.syntax/bsv05/interface::ModuleInterface_LocalStmt_DeclAssignLet.bsv",
        "testsuite/bsc.syntax/bsv05/interface",
        "ModuleInterface_LocalStmt_DeclAssignLet.bsv"
    ),
    frontend_pass!(
        "bsc.syntax/bsv05/interface::ModuleInterface_LocalStmt_DeclFunction.bsv",
        "testsuite/bsc.syntax/bsv05/interface",
        "ModuleInterface_LocalStmt_DeclFunction.bsv"
    ),
    frontend_pass!(
        "bsc.syntax/bsv05/interface::ModuleInterface_LocalStmt_DeclLet_Assign.bsv",
        "testsuite/bsc.syntax/bsv05/interface",
        "ModuleInterface_LocalStmt_DeclLet_Assign.bsv"
    ),
    frontend_pass!(
        "bsc.syntax/bsv05/interface::ModuleInterface_LocalStmt_DeclModule.bsv",
        "testsuite/bsc.syntax/bsv05/interface",
        "ModuleInterface_LocalStmt_DeclModule.bsv"
    ),
    frontend_pass!(
        "bsc.syntax/bsv05/interface::ModuleInterface_LocalStmt_Decl_Assign.bsv",
        "testsuite/bsc.syntax/bsv05/interface",
        "ModuleInterface_LocalStmt_Decl_Assign.bsv"
    ),
    frontend_pass!(
        "bsc.syntax/bsv05/interface::ModuleInterface_LocalStmt_DeclArray_ForLoop.bsv",
        "testsuite/bsc.syntax/bsv05/interface",
        "ModuleInterface_LocalStmt_DeclArray_ForLoop.bsv"
    ),
    frontend_pass!(
        "bsc.syntax/bsv05/interface::ModuleInterface_LocalStmt_DeclAssign_If.bsv",
        "testsuite/bsc.syntax/bsv05/interface",
        "ModuleInterface_LocalStmt_DeclAssign_If.bsv"
    ),
    frontend_pass!(
        "bsc.syntax/bsv05/interface::ModuleInterface_LocalStmt_DeclAssign_Case.bsv",
        "testsuite/bsc.syntax/bsv05/interface",
        "ModuleInterface_LocalStmt_DeclAssign_Case.bsv"
    ),
    frontend_pass!(
        "bsc.syntax/bsv05/interface::ModuleInterface_LocalStmt_DeclArrayAssign_AssignSub.bsv",
        "testsuite/bsc.syntax/bsv05/interface",
        "ModuleInterface_LocalStmt_DeclArrayAssign_AssignSub.bsv"
    ),
    frontend_pass!(
        "bsc.syntax/bsv05/interface::ModuleInterface_LocalStmt_Decl_AssignField.bsv",
        "testsuite/bsc.syntax/bsv05/interface",
        "ModuleInterface_LocalStmt_Decl_AssignField.bsv"
    ),
    frontend_pass!(
        "bsc.syntax/bsv05/interface::PrimaryInterface_LocalStmt_DeclAssign.bsv",
        "testsuite/bsc.syntax/bsv05/interface",
        "PrimaryInterface_LocalStmt_DeclAssign.bsv"
    ),
    frontend_pass!(
        "bsc.syntax/bsv05/interface::PrimaryInterface_LocalStmt_DeclAssignLet.bsv",
        "testsuite/bsc.syntax/bsv05/interface",
        "PrimaryInterface_LocalStmt_DeclAssignLet.bsv"
    ),
    frontend_pass!(
        "bsc.syntax/bsv05/interface::PrimaryInterface_LocalStmt_DeclFunction.bsv",
        "testsuite/bsc.syntax/bsv05/interface",
        "PrimaryInterface_LocalStmt_DeclFunction.bsv"
    ),
    frontend_pass!(
        "bsc.syntax/bsv05/interface::PrimaryInterface_LocalStmt_DeclLet_Assign.bsv",
        "testsuite/bsc.syntax/bsv05/interface",
        "PrimaryInterface_LocalStmt_DeclLet_Assign.bsv"
    ),
    frontend_pass!(
        "bsc.syntax/bsv05/interface::PrimaryInterface_LocalStmt_DeclModule.bsv",
        "testsuite/bsc.syntax/bsv05/interface",
        "PrimaryInterface_LocalStmt_DeclModule.bsv"
    ),
    frontend_pass!(
        "bsc.syntax/bsv05/interface::PrimaryInterface_LocalStmt_Decl_Assign.bsv",
        "testsuite/bsc.syntax/bsv05/interface",
        "PrimaryInterface_LocalStmt_Decl_Assign.bsv"
    ),
    frontend_pass!(
        "bsc.syntax/bsv05/interface::PrimaryInterface_LocalStmt_DeclArray_ForLoop.bsv",
        "testsuite/bsc.syntax/bsv05/interface",
        "PrimaryInterface_LocalStmt_DeclArray_ForLoop.bsv"
    ),
    frontend_pass!(
        "bsc.syntax/bsv05/interface::PrimaryInterface_LocalStmt_DeclAssign_If.bsv",
        "testsuite/bsc.syntax/bsv05/interface",
        "PrimaryInterface_LocalStmt_DeclAssign_If.bsv"
    ),
    frontend_pass!(
        "bsc.syntax/bsv05/interface::PrimaryInterface_LocalStmt_DeclAssign_Case.bsv",
        "testsuite/bsc.syntax/bsv05/interface",
        "PrimaryInterface_LocalStmt_DeclAssign_Case.bsv"
    ),
    frontend_pass!(
        "bsc.syntax/bsv05/interface::PrimaryInterface_LocalStmt_DeclArrayAssign_AssignSub.bsv",
        "testsuite/bsc.syntax/bsv05/interface",
        "PrimaryInterface_LocalStmt_DeclArrayAssign_AssignSub.bsv"
    ),
    frontend_pass!(
        "bsc.syntax/bsv05/interface::PrimaryInterface_LocalStmt_Decl_AssignField.bsv",
        "testsuite/bsc.syntax/bsv05/interface",
        "PrimaryInterface_LocalStmt_Decl_AssignField.bsv"
    ),
    frontend_fail_with_options!(
        "bsc.syntax/bsv05/interface::ModuleInterface_LocalStmt_NoDecl.bsv",
        "testsuite/bsc.syntax/bsv05/interface",
        "ModuleInterface_LocalStmt_NoDecl.bsv",
        &["P0039"]
    ),
    frontend_fail_with_options!(
        "bsc.syntax/bsv05/interface::PrimaryInterface_LocalStmt_NoDecl.bsv",
        "testsuite/bsc.syntax/bsv05/interface",
        "PrimaryInterface_LocalStmt_NoDecl.bsv",
        &["P0039"]
    ),
    frontend_pass!(
        "bsc.syntax/bsv05/interface::ModuleInterface_LocalStmt_NoFields.bsv",
        "testsuite/bsc.syntax/bsv05/interface",
        "ModuleInterface_LocalStmt_NoFields.bsv"
    ),
    frontend_pass!(
        "bsc.syntax/bsv05/interface::PrimaryInterface_LocalStmt_NoFields.bsv",
        "testsuite/bsc.syntax/bsv05/interface",
        "PrimaryInterface_LocalStmt_NoFields.bsv"
    ),
    frontend_fail_with_options!(
        "bsc.syntax/bsv05/interface::ModuleInterface_LocalStmt_Between.bsv",
        "testsuite/bsc.syntax/bsv05/interface",
        "ModuleInterface_LocalStmt_Between.bsv",
        &["P0032"]
    ),
    frontend_fail_with_options!(
        "bsc.syntax/bsv05/interface::PrimaryInterface_LocalStmt_Between.bsv",
        "testsuite/bsc.syntax/bsv05/interface",
        "PrimaryInterface_LocalStmt_Between.bsv",
        &["P0032"]
    ),
    // testsuite/bsc.typechecker/assignment/assignment.exp (18)
    frontend_pass!(
        "bsc.typechecker/assignment::ListNSelectNoParens.bsv",
        "testsuite/bsc.typechecker/assignment",
        "ListNSelectNoParens.bsv"
    ),
    frontend_pass!(
        "bsc.typechecker/assignment::ListNSelectParens.bsv",
        "testsuite/bsc.typechecker/assignment",
        "ListNSelectParens.bsv"
    ),
    frontend_pass!(
        "bsc.typechecker/assignment::ListSelectNoParens.bsv",
        "testsuite/bsc.typechecker/assignment",
        "ListSelectNoParens.bsv"
    ),
    frontend_pass!(
        "bsc.typechecker/assignment::ListSelectParens.bsv",
        "testsuite/bsc.typechecker/assignment",
        "ListSelectParens.bsv"
    ),
    frontend_pass!(
        "bsc.typechecker/assignment::VectorSelectNoParens.bsv",
        "testsuite/bsc.typechecker/assignment",
        "VectorSelectNoParens.bsv"
    ),
    frontend_pass!(
        "bsc.typechecker/assignment::VectorSelectParens.bsv",
        "testsuite/bsc.typechecker/assignment",
        "VectorSelectParens.bsv"
    ),
    frontend_pass!(
        "bsc.typechecker/assignment::PrimArraySelectNoParens.bsv",
        "testsuite/bsc.typechecker/assignment",
        "PrimArraySelectNoParens.bsv"
    ),
    frontend_pass!(
        "bsc.typechecker/assignment::PrimArraySelectParens.bsv",
        "testsuite/bsc.typechecker/assignment",
        "PrimArraySelectParens.bsv"
    ),
    frontend_pass!(
        "bsc.typechecker/assignment::ListSelect2NoParens.bsv",
        "testsuite/bsc.typechecker/assignment",
        "ListSelect2NoParens.bsv"
    ),
    frontend_pass!(
        "bsc.typechecker/assignment::ListSelect2Parens.bsv",
        "testsuite/bsc.typechecker/assignment",
        "ListSelect2Parens.bsv"
    ),
    frontend_fail_error_with_options!(
        "bsc.typechecker/assignment::ListMissingSelectNoParens.bsv",
        "testsuite/bsc.typechecker/assignment",
        "ListMissingSelectNoParens.bsv",
        "T0066",
        &[]
    ),
    frontend_fail_error_with_options!(
        "bsc.typechecker/assignment::ListMissingSelectParens.bsv",
        "testsuite/bsc.typechecker/assignment",
        "ListMissingSelectParens.bsv",
        "T0066",
        &[]
    ),
    frontend_pass!(
        "bsc.typechecker/assignment::VectorUpdateNoParens.bsv",
        "testsuite/bsc.typechecker/assignment",
        "VectorUpdateNoParens.bsv"
    ),
    frontend_pass!(
        "bsc.typechecker/assignment::VectorUpdateParens.bsv",
        "testsuite/bsc.typechecker/assignment",
        "VectorUpdateParens.bsv"
    ),
    frontend_pass!(
        "bsc.typechecker/assignment::StructRegWrite.bsv",
        "testsuite/bsc.typechecker/assignment",
        "StructRegWrite.bsv"
    ),
    frontend_pass!(
        "bsc.typechecker/assignment::RegStructWrite.bsv::frontend",
        "testsuite/bsc.typechecker/assignment",
        "RegStructWrite.bsv"
    ),
    verilog_fail_error!(
        "bsc.typechecker/assignment::RegStructWrite.bsv::verilog",
        "testsuite/bsc.typechecker/assignment",
        "RegStructWrite.bsv",
        "G0004"
    ),
    frontend_fail_with_options!(
        "bsc.typechecker/assignment::StructUpdReg.bsv",
        "testsuite/bsc.typechecker/assignment",
        "StructUpdReg.bsv",
        &["T0015"]
    ),
    // testsuite/bsc.interra/messages/EBadIfcType/EBadIfcType.exp (5)
    frontend_fail_error_with_options!(
        "bsc.interra/messages/EBadIfcType::EBadIfcType_context.bs",
        "testsuite/bsc.interra/messages/EBadIfcType",
        "EBadIfcType_context.bs",
        "T0043",
        &["-verilog", "-g", "mkFib"]
    ),
    frontend_fail_error_with_options!(
        "bsc.interra/messages/EBadIfcType::EBadIfcType_field.bs",
        "testsuite/bsc.interra/messages/EBadIfcType",
        "EBadIfcType_field.bs",
        "T0043",
        &["-verilog", "-g", "add"]
    ),
    frontend_fail_error_with_options!(
        "bsc.interra/messages/EBadIfcType::EBadIfcType_interface.bs",
        "testsuite/bsc.interra/messages/EBadIfcType",
        "EBadIfcType_interface.bs",
        "T0043",
        &["-verilog", "-g", "mkNull"]
    ),
    frontend_fail_error_with_options!(
        "bsc.interra/messages/EBadIfcType::EBadIfcType_module.bs",
        "testsuite/bsc.interra/messages/EBadIfcType",
        "EBadIfcType_module.bs",
        "T0043",
        &["-verilog", "-g", "function_GCD"]
    ),
    frontend_fail_error_with_options!(
        "bsc.interra/messages/EBadIfcType::EBadIfcType_polymorphic.bs",
        "testsuite/bsc.interra/messages/EBadIfcType",
        "EBadIfcType_polymorphic.bs",
        "T0043",
        &["-verilog", "-g", "mkFib"]
    ),
    // testsuite/bsc.preprocessor/ifdef/ifdef.exp (5)
    frontend_pass!(
        "bsc.preprocessor/ifdef::bug1190.bsv",
        "testsuite/bsc.preprocessor/ifdef",
        "bug1190.bsv"
    ),
    frontend_pass!(
        "bsc.preprocessor/ifdef::ifdef2556.bsv",
        "testsuite/bsc.preprocessor/ifdef",
        "ifdef2556.bsv"
    ),
    frontend_pass!(
        "bsc.preprocessor/ifdef::ifdef4891.bsv",
        "testsuite/bsc.preprocessor/ifdef",
        "ifdef4891.bsv"
    ),
    frontend_pass!(
        "bsc.preprocessor/ifdef::ifdef7672.bsv",
        "testsuite/bsc.preprocessor/ifdef",
        "ifdef7672.bsv"
    ),
    frontend_pass!(
        "bsc.preprocessor/ifdef::ifdef7720.bsv",
        "testsuite/bsc.preprocessor/ifdef",
        "ifdef7720.bsv"
    ),
    // testsuite/bsc.driver/symtab/symtab.exp (4)
    frontend_pass!(
        "bsc.driver/symtab::FieldDup.bsv",
        "testsuite/bsc.driver/symtab",
        "FieldDup.bsv",
        &["FieldDup.bsv", "FieldDup_Wrapper.bsv", "FieldDup_Leaf.bsv"]
    ),
    frontend_pass!(
        "bsc.driver/symtab::ConDup.bsv",
        "testsuite/bsc.driver/symtab",
        "ConDup.bsv",
        &["ConDup.bsv", "ConDup_Wrapper.bsv", "ConDup_Leaf.bsv"]
    ),
    frontend_pass!(
        "bsc.driver/symtab::TypeclassDup.bsv",
        "testsuite/bsc.driver/symtab",
        "TypeclassDup.bsv",
        &[
            "TypeclassDup.bsv",
            "TypeclassDup_Wrapper.bsv",
            "TypeclassDup_Leaf.bsv",
        ]
    ),
    frontend_pass!(
        "bsc.driver/symtab::TypeclassDupSuperAbstract.bsv",
        "testsuite/bsc.driver/symtab",
        "TypeclassDupSuperAbstract.bsv",
        &[
            "TypeclassDupSuperAbstract.bsv",
            "TypeclassDupSuperAbstract_Wrapper.bsv",
            "TypeclassDupSuperAbstract_Leaf.bsv",
        ]
    ),
];
