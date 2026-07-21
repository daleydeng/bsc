{-# LANGUAGE CPP #-}
module SATPred(
  SATPredState,
  initSATPredState,
  solvePred
  ) where

import Flags
import Pred

#ifndef BSC_Z3_ONLY
import qualified Pred2STP as STP
         (SState, initSState, solvePred)
import qualified Pred2Yices as Yices
         (YState, initYState, solvePred)
#endif
import qualified Pred2Z3 as Z3
         (ZState, initZState, solvePred)

-- -------------------------

-- A single data type for any of the solver state
data SATPredState =
#ifndef BSC_Z3_ONLY
           SATPredS_STP STP.SState
         | SATPredS_Yices Yices.YState
         |
#endif
           SATPredS_Z3 Z3.ZState

-- -------------------------

initSATPredState :: Flags -> IO SATPredState
initSATPredState flags = do
    case (satBackend flags) of
#ifndef BSC_Z3_ONLY
      SAT_STP -> do
        stp_state <- STP.initSState
        return (SATPredS_STP stp_state)
      SAT_Yices -> do
        yices_state <- Yices.initYState
        return (SATPredS_Yices yices_state)
#else
      SAT_STP -> unavailable "STP"
      SAT_Yices -> unavailable "Yices"
#endif
      SAT_Z3 -> do
        z3_state <- Z3.initZState
        return (SATPredS_Z3 z3_state)
#ifdef BSC_Z3_ONLY
  where unavailable name = ioError (userError (name ++
                             " is not compiled into this BSC build; use -sat-z3"))
#endif

-- -------------------------

{-
checkPreds :: SATPredState -> [Pred] -> IO ([EMsg], SATPredState)
checkPreds (SATPredS_STP stp_state) ps = do
    (res, stp_state') <- STP.checkPreds stp_state ps
    return (res, SATPredS_STP stp_state')
checkPreds (SATPredS_Yices yices_state) ps = do
    (res, yices_state') <- Yices.checkPreds yices_state ps
    return (res, SATPredS_Yices yices_state')
-}

-- -------------------------

solvePred :: SATPredState -> [Pred] -> Pred -> IO (Maybe Pred, SATPredState)
#ifndef BSC_Z3_ONLY
solvePred (SATPredS_STP stp_state) ps p = do
    (res, stp_state') <- STP.solvePred stp_state ps p
    return (res, SATPredS_STP stp_state')
solvePred (SATPredS_Yices yices_state) ps p = do
    (res, yices_state') <- Yices.solvePred yices_state ps p
    return (res, SATPredS_Yices yices_state')
#endif
solvePred (SATPredS_Z3 z3_state) ps p = do
    (res, z3_state') <- Z3.solvePred z3_state ps p
    return (res, SATPredS_Z3 z3_state')

-- -------------------------
