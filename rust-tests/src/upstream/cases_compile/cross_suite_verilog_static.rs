//! Origins:
//! - `testsuite/bsc.mcd/Pragmas/Pragmas.exp`
//! - `testsuite/bsc.interra/Path_Analysis/Single_Module/Single_Module.exp`
//! - `testsuite/bsc.interra/Path_Analysis/Extended_Input_Output_Path/Extended_Input_Output_Path.exp`
//! - `testsuite/bsc.names/portRenaming/conflicts/modarg/modarg.exp`
//! - `testsuite/bsc.names/portRenaming/conflicts/modparam/modparam.exp`
//! - `testsuite/bsc.names/portRenaming/invalidAttrs/port/port.exp`
//! - `testsuite/bsc.names/portRenaming/invalidAttrs/enable/enable.exp`
//! - `testsuite/bsc.names/portRenaming/invalidAttrs/ready/ready.exp`
//! - `testsuite/bsc.names/portRenaming/invalidAttrs/result/result.exp`

use super::CompileCase;

macro_rules! verilog_pass {
    ($prefix:literal, $fixture_dir:literal, $source:literal) => {
        compile_verilog_pass_case!(concat!($prefix, "::", $source), $fixture_dir, $source)
    };
}

macro_rules! verilog_error {
    ($prefix:literal, $fixture_dir:literal, $source:literal, $tag:literal $(, $count:expr)?) => {
        compile_verilog_fail_error_case!(
            concat!($prefix, "::", $source),
            $fixture_dir,
            $source,
            $tag
            $(, $count)?
        )
    };
}

macro_rules! pass_case {
    (pragmas, $source:literal) => {
        verilog_pass!("bsc.mcd/Pragmas", "testsuite/bsc.mcd/Pragmas", $source)
    };
    (modarg, $source:literal) => {
        verilog_pass!(
            "bsc.names/portRenaming/conflicts/modarg",
            "testsuite/bsc.names/portRenaming/conflicts/modarg",
            $source
        )
    };
    (modparam, $source:literal) => {
        verilog_pass!(
            "bsc.names/portRenaming/conflicts/modparam",
            "testsuite/bsc.names/portRenaming/conflicts/modparam",
            $source
        )
    };
}

macro_rules! error_case {
    (pragmas, $source:literal, $tag:literal $(, $count:expr)?) => {
        verilog_error!(
            "bsc.mcd/Pragmas",
            "testsuite/bsc.mcd/Pragmas",
            $source,
            $tag
            $(, $count)?
        )
    };
    (single, $source:literal, $tag:literal $(, $count:expr)?) => {
        verilog_error!(
            "bsc.interra/Path_Analysis/Single_Module",
            "testsuite/bsc.interra/Path_Analysis/Single_Module",
            $source,
            $tag
            $(, $count)?
        )
    };
    (extended, $source:literal, $tag:literal $(, $count:expr)?) => {
        verilog_error!(
            "bsc.interra/Path_Analysis/Extended_Input_Output_Path",
            "testsuite/bsc.interra/Path_Analysis/Extended_Input_Output_Path",
            $source,
            $tag
            $(, $count)?
        )
    };
    (modarg, $source:literal, $tag:literal $(, $count:expr)?) => {
        verilog_error!(
            "bsc.names/portRenaming/conflicts/modarg",
            "testsuite/bsc.names/portRenaming/conflicts/modarg",
            $source,
            $tag
            $(, $count)?
        )
    };
    (modparam, $source:literal, $tag:literal $(, $count:expr)?) => {
        verilog_error!(
            "bsc.names/portRenaming/conflicts/modparam",
            "testsuite/bsc.names/portRenaming/conflicts/modparam",
            $source,
            $tag
            $(, $count)?
        )
    };
    (port, $source:literal, $tag:literal $(, $count:expr)?) => {
        verilog_error!(
            "bsc.names/portRenaming/invalidAttrs/port",
            "testsuite/bsc.names/portRenaming/invalidAttrs/port",
            $source,
            $tag
            $(, $count)?
        )
    };
    (enable, $source:literal, $tag:literal $(, $count:expr)?) => {
        verilog_error!(
            "bsc.names/portRenaming/invalidAttrs/enable",
            "testsuite/bsc.names/portRenaming/invalidAttrs/enable",
            $source,
            $tag
            $(, $count)?
        )
    };
    (ready, $source:literal, $tag:literal $(, $count:expr)?) => {
        verilog_error!(
            "bsc.names/portRenaming/invalidAttrs/ready",
            "testsuite/bsc.names/portRenaming/invalidAttrs/ready",
            $source,
            $tag
            $(, $count)?
        )
    };
    (result, $source:literal, $tag:literal $(, $count:expr)?) => {
        verilog_error!(
            "bsc.names/portRenaming/invalidAttrs/result",
            "testsuite/bsc.names/portRenaming/invalidAttrs/result",
            $source,
            $tag
            $(, $count)?
        )
    };
}

pub(super) const CASES: &[CompileCase] = &[
    // testsuite/bsc.mcd/Pragmas/Pragmas.exp
    pass_case!(pragmas, "SameFamilyIn.bsv"),
    pass_case!(pragmas, "SameFamilyDefault.bsv"),
    pass_case!(pragmas, "AncestorsBoundary.bsv"),
    error_case!(pragmas, "CheckSameFamily.bsv", "G0037"),
    error_case!(pragmas, "CheckAncestors.bsv", "G0086"),
    error_case!(pragmas, "AncestorTest1.bsv", "G0086"),
    error_case!(pragmas, "AncestorTest2.bsv", "G0086"),
    pass_case!(pragmas, "AncestorTest3.bsv"),
    error_case!(pragmas, "AncestorTest4.bsv", "G0086"),
    error_case!(pragmas, "AncestorTest5.bsv", "G0086"),
    pass_case!(pragmas, "AncestorTest6.bsv"),
    pass_case!(pragmas, "AncestorTest7.bsv"),
    pass_case!(pragmas, "AncestorTest8.bsv"),
    error_case!(pragmas, "FamilyTest1.bsv", "G0037"),
    pass_case!(pragmas, "FamilyTest2.bsv"),
    error_case!(pragmas, "SameFamilyError1.bsv", "P0182"),
    error_case!(pragmas, "AncestorsError1.bsv", "P0005"),
    error_case!(pragmas, "AncestorsError2.bsv", "P0063"),
    error_case!(pragmas, "AncestorsError3.bsv", "P0005"),
    error_case!(pragmas, "AncestorsError4.bsv", "P0005"),
    error_case!(pragmas, "EmptyCLKAttrib.bsv", "P0177"),
    error_case!(pragmas, "EmptyGATEAttrib.bsv", "P0177"),
    error_case!(pragmas, "EmptyRSTNAttrib.bsv", "P0177"),
    error_case!(pragmas, "CLKAttribVerilogKeyword.bsv", "P0184"),
    error_case!(pragmas, "CLKAttribWithSpace.bsv", "P0185"),
    error_case!(pragmas, "EUseDefaultClock.bsv", "G0118"),
    error_case!(pragmas, "EUseDefaultReset.bsv", "G0119"),
    pass_case!(pragmas, "EUseDefaultReset_OK.bsv"),
    // testsuite/bsc.interra/Path_Analysis/Single_Module/Single_Module.exp
    error_case!(single, "ArgMethod2ReturnValue2.bsv", "G0032"),
    error_case!(single, "ArgMethod2ReturnValue3.bsv", "G0032"),
    error_case!(single, "ArgMethod2ReturnValue.bsv", "G0032"),
    error_case!(single, "Argument2Rdy2.bsv", "G0033"),
    error_case!(single, "Argument2Rdy.bsv", "G0033", 2),
    error_case!(single, "Argument2ReturnValue2.bsv", "G0032"),
    error_case!(single, "Argument2ReturnValue3.bsv", "G0032"),
    error_case!(single, "Argument2ReturnValue.bsv", "G0032"),
    error_case!(single, "Combo_Loop.bsv", "G0035", 2),
    error_case!(single, "En2Rdy2.bsv", "G0030"),
    error_case!(single, "En2Rdy4.bsv", "G0033", 2),
    error_case!(single, "En2Rdy.bsv", "G0033"),
    error_case!(single, "En2ReturnValue2.bsv", "G0033"),
    error_case!(single, "En2ReturnValue.bsv", "G0033"),
    error_case!(single, "EnableSignal2ReturnValue.bsv", "G0033"),
    compile_verilog_fail_error_case!(
        "bsc.interra/Path_Analysis/Single_Module::MuxLogic.bsv::G0034",
        "testsuite/bsc.interra/Path_Analysis/Single_Module",
        "MuxLogic.bsv",
        "G0034"
    ),
    compile_verilog_fail_error_case!(
        "bsc.interra/Path_Analysis/Single_Module::MuxLogic.bsv::G0035",
        "testsuite/bsc.interra/Path_Analysis/Single_Module",
        "MuxLogic.bsv",
        "G0035"
    ),
    error_case!(single, "Parameter2Rdy.bsv", "G0033"),
    error_case!(single, "Parameter2ReturnValue.bsv", "G0032"),
    error_case!(single, "RWireReadB4Write.bsv", "G0033"),
    error_case!(single, "WillFire2CanFire.bsv", "G0033"),
    // testsuite/bsc.interra/Path_Analysis/Extended_Input_Output_Path/Extended_Input_Output_Path.exp
    error_case!(extended, "Argument2Rdy.bsv", "G0033", 2),
    error_case!(extended, "Argument2ReturnValue2.bsv", "G0032"),
    error_case!(extended, "Argument2ReturnValue3.bsv", "G0032"),
    error_case!(extended, "Argument2ReturnValue.bsv", "G0032"),
    error_case!(extended, "En2Rdy2.bsv", "G0030"),
    error_case!(extended, "En2Rdy.bsv", "G0033"),
    error_case!(extended, "En2ReturnValue2.bsv", "G0033"),
    error_case!(extended, "En2ReturnValue.bsv", "G0033"),
    error_case!(extended, "Ten_Inverters.bsv", "G0032"),
    // testsuite/bsc.names/portRenaming/conflicts/modarg/modarg.exp
    error_case!(modarg, "ModargResult.bsv", "G0107"),
    error_case!(modarg, "ModargEnable.bsv", "G0107"),
    error_case!(modarg, "ModargReady.bsv", "G0107"),
    error_case!(modarg, "ModargPort.bsv", "G0107"),
    error_case!(modarg, "ModargPortRename.bsv", "G0107"),
    error_case!(modarg, "ModargClock.bsv", "G0107"),
    error_case!(modarg, "ModargGate.bsv", "G0107"),
    error_case!(modarg, "ModargClockPrefix.bsv", "G0107"),
    error_case!(modarg, "ModargGatePrefix.bsv", "G0107"),
    pass_case!(modarg, "ModargClockPrefixOK.bsv"),
    pass_case!(modarg, "ModargGatePrefixOK.bsv"),
    error_case!(modarg, "ModargReset.bsv", "G0107"),
    error_case!(modarg, "ModargResetPrefix.bsv", "G0107"),
    pass_case!(modarg, "ModargResetPrefixOK.bsv"),
    error_case!(modarg, "ModargInout.bsv", "G0107"),
    error_case!(modarg, "ModargInoutPrefix.bsv", "G0107"),
    pass_case!(modarg, "ModargInoutPrefixOK.bsv"),
    error_case!(modarg, "ModargInoutRename.bsv", "G0107"),
    // testsuite/bsc.names/portRenaming/conflicts/modparam/modparam.exp
    error_case!(modparam, "ModparamResult.bsv", "G0107"),
    error_case!(modparam, "ModparamEnable.bsv", "G0107"),
    error_case!(modparam, "ModparamReady.bsv", "G0107"),
    error_case!(modparam, "ModparamPort.bsv", "G0107"),
    error_case!(modparam, "ModparamPortRename.bsv", "G0107"),
    error_case!(modparam, "ModparamClock.bsv", "G0107"),
    error_case!(modparam, "ModparamGate.bsv", "G0107"),
    error_case!(modparam, "ModparamClockPrefix.bsv", "G0107"),
    error_case!(modparam, "ModparamGatePrefix.bsv", "G0107"),
    pass_case!(modparam, "ModparamClockPrefixOK.bsv"),
    pass_case!(modparam, "ModparamGatePrefixOK.bsv"),
    error_case!(modparam, "ModparamReset.bsv", "G0107"),
    error_case!(modparam, "ModparamResetPrefix.bsv", "G0107"),
    pass_case!(modparam, "ModparamResetPrefixOK.bsv"),
    error_case!(modparam, "ModparamInout.bsv", "G0107"),
    error_case!(modparam, "ModparamInoutPrefix.bsv", "G0107"),
    pass_case!(modparam, "ModparamInoutPrefixOK.bsv"),
    error_case!(modparam, "ModparamInoutRename.bsv", "G0107"),
    // testsuite/bsc.names/portRenaming/invalidAttrs/port/port.exp
    error_case!(port, "Keyword.bsv", "G0105"),
    error_case!(port, "InvalidName.bsv", "G0106"),
    error_case!(port, "Empty.bsv", "P0157"),
    error_case!(port, "Space.bsv", "P0157"),
    error_case!(port, "DuplicateValue.bsv", "P0086"),
    error_case!(port, "DuplicateAttr.bsv", "P0156"),
    error_case!(port, "WrongLoc_Method.bsv", "P0155"),
    // testsuite/bsc.names/portRenaming/invalidAttrs/enable/enable.exp
    error_case!(enable, "Keyword.bsv", "G0105"),
    error_case!(enable, "InvalidName.bsv", "G0106"),
    error_case!(enable, "Empty.bsv", "P0157"),
    error_case!(enable, "Space.bsv", "P0157"),
    // testsuite/bsc.names/portRenaming/invalidAttrs/ready/ready.exp
    error_case!(ready, "Keyword.bsv", "G0105"),
    error_case!(ready, "InvalidName.bsv", "G0106"),
    error_case!(ready, "Empty.bsv", "P0157"),
    error_case!(ready, "Space.bsv", "P0157"),
    // testsuite/bsc.names/portRenaming/invalidAttrs/result/result.exp
    error_case!(result, "Keyword.bsv", "G0105"),
    error_case!(result, "InvalidName.bsv", "G0106"),
    error_case!(result, "Empty.bsv", "P0157"),
    error_case!(result, "Space.bsv", "P0157"),
];
