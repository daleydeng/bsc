{-# LANGUAGE CPP #-}
module SAT(
           SATState,
           initSATState,
           checkBiImplication,
           isConstExpr,
           checkEq,
           checkNotEq,
           checkSATFlags
          ) where

import qualified Control.Exception as CE

import Error(ErrorHandle, bsError, ErrMsg(..))
import Flags
import ASyntax
import Position(cmdPosition)

#ifndef BSC_Z3_ONLY
import qualified AExpr2STP as STP
         (SState, initSState, checkBiImplication, isConstExpr,
          checkEq, checkNotEq)
import qualified AExpr2Yices as Yices
         (YState, initYState, checkBiImplication, isConstExpr,
          checkEq, checkNotEq)

import STP(checkVersion)
import Yices(checkVersion)
#endif
import qualified AExpr2Z3 as Z3
         (ZState, initZState, checkBiImplication, isConstExpr,
          checkEq, checkNotEq)
import qualified SMTLib2Z3 as Z3SMT(checkVersion)

-- -------------------------

-- A single data type for either of the solver state
data SATState =
#ifndef BSC_Z3_ONLY
                SATS_Yices Yices.YState
              | SATS_STP STP.SState
              |
#endif
                SATS_Z3 Z3.ZState

-- -------------------------

initSATState :: String -> ErrorHandle -> Flags -> Bool -> [ADef] -> [AVInst] ->
                IO SATState
initSATState str errh flags doHardFail ds avis =
    case (satBackend flags) of
#ifndef BSC_Z3_ONLY
      SAT_Yices -> do
          yices_state <- Yices.initYState str flags doHardFail ds avis []
          return (SATS_Yices yices_state)
      SAT_STP -> do
          stp_state <- STP.initSState str flags doHardFail ds avis []
          return (SATS_STP stp_state)

#else
      SAT_Yices -> unavailable "Yices"
      SAT_STP -> unavailable "STP"
#endif
      SAT_Z3 -> do
          z3_state <- Z3.initZState str flags doHardFail ds avis []
          return (SATS_Z3 z3_state)
#ifdef BSC_Z3_ONLY
  where unavailable name = ioError (userError (name ++
                             " is not compiled into this BSC build; use -sat-z3"))
#endif

checkSATFlags :: ErrorHandle -> Flags -> IO Flags
checkSATFlags eh f =
  let
#ifndef BSC_Z3_ONLY
      hasYices :: IO Bool
      hasYices = let handler :: CE.SomeException -> IO Bool
                     handler _ = return False
                 in  CE.catch (Yices.checkVersion >> return True) handler

      hasSTP :: IO Bool
      hasSTP = STP.checkVersion
#endif

      hasZ3 :: IO Bool
      hasZ3 = let handler :: CE.SomeException -> IO Bool
                  handler _ = return False
              in  CE.catch (Z3SMT.checkVersion >> return True) handler

      checkFn :: String -> String -> IO Bool -> IO Flags
      checkFn flag_str lib_str hasFn = do
        res <- hasFn
        if res
          then return f
          else -- Rather than defaulting to another solver,
               -- just report an error
               bsError eh [(cmdPosition,
                            WSATNotAvailable flag_str lib_str Nothing)]
  in  case (satBackend f) of
#ifndef BSC_Z3_ONLY
        SAT_Yices -> checkFn "-sat-yices" "libyices.so.2" hasYices
        SAT_STP -> checkFn "-sat-stp" "libstp.so" hasSTP
#else
        SAT_Yices -> checkFn "-sat-yices" "Yices (not compiled in)" (return False)
        SAT_STP -> checkFn "-sat-stp" "STP (not compiled in)" (return False)
#endif
        SAT_Z3 -> checkFn "-sat-z3" "z3" hasZ3

-- -------------------------

checkBiImplication :: SATState -> AExpr -> AExpr -> IO ((Bool, Bool), SATState)
#ifndef BSC_Z3_ONLY
checkBiImplication (SATS_Yices yices_state) e1 e2 = do
    (res, yices_state') <- Yices.checkBiImplication yices_state e1 e2
    return (res, SATS_Yices yices_state')
checkBiImplication (SATS_STP stp_state) e1 e2 = do
    (res, stp_state') <- STP.checkBiImplication stp_state e1 e2
    return (res, SATS_STP stp_state')

#endif
checkBiImplication (SATS_Z3 z3_state) e1 e2 = do
    (res, z3_state') <- Z3.checkBiImplication z3_state e1 e2
    return (res, SATS_Z3 z3_state')

isConstExpr :: SATState -> AExpr -> IO (Maybe Bool, SATState)
#ifndef BSC_Z3_ONLY
isConstExpr (SATS_Yices yices_state) e = do
    (res, yices_state') <- Yices.isConstExpr yices_state e
    return (res, SATS_Yices yices_state')
isConstExpr (SATS_STP stp_state) e = do
    (res, stp_state') <- STP.isConstExpr stp_state e
    return (res, SATS_STP stp_state')

#endif
isConstExpr (SATS_Z3 z3_state) e = do
    (res, z3_state') <- Z3.isConstExpr z3_state e
    return (res, SATS_Z3 z3_state')

checkEq :: SATState -> AExpr -> AExpr -> IO (Maybe Bool, SATState)
#ifndef BSC_Z3_ONLY
checkEq (SATS_Yices yices_state) e1 e2 = do
    (res, yices_state') <- Yices.checkEq yices_state e1 e2
    return (res, SATS_Yices yices_state')
checkEq (SATS_STP stp_state) e1 e2 = do
    (res, stp_state') <- STP.checkEq stp_state e1 e2
    return (res, SATS_STP stp_state')

#endif
checkEq (SATS_Z3 z3_state) e1 e2 = do
    (res, z3_state') <- Z3.checkEq z3_state e1 e2
    return (res, SATS_Z3 z3_state')

checkNotEq :: SATState -> AExpr -> AExpr -> IO (Maybe Bool, SATState)
#ifndef BSC_Z3_ONLY
checkNotEq (SATS_Yices yices_state) e1 e2 = do
    (res, yices_state') <- Yices.checkNotEq yices_state e1 e2
    return (res, SATS_Yices yices_state')
checkNotEq (SATS_STP stp_state) e1 e2 = do
    (res, stp_state') <- STP.checkNotEq stp_state e1 e2
    return (res, SATS_STP stp_state')

#endif
checkNotEq (SATS_Z3 z3_state) e1 e2 = do
    (res, z3_state') <- Z3.checkNotEq z3_state e1 e2
    return (res, SATS_Z3 z3_state')

-- -------------------------
