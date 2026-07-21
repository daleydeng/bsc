module SMTLib2Z3(
       SMTResult(..),
       checkVersion,
       runZ3
) where

import Data.Char(isSpace)
import System.Directory(findExecutable)
import System.Exit(ExitCode(..))
import System.Process(readProcessWithExitCode)

-- Keep the solver result distinct from Bool: "unknown" must never be
-- mistaken for either a proof or a counterexample.
data SMTResult = SMTSat | SMTUnsat | SMTUnknown
                 deriving (Eq, Ord, Show)

findZ3 :: IO FilePath
findZ3 = do
  mpath <- findExecutable "z3"
  case mpath of
    Just path -> return path
    Nothing -> ioError (userError "Z3 executable was not found on PATH")

checkVersion :: IO String
checkVersion = do
  exe <- findZ3
  (status, out, err) <- readProcessWithExitCode exe ["-version"] ""
  case status of
    ExitSuccess -> return (trim (if null out then err else out))
    ExitFailure n ->
      ioError (userError ("z3 -version failed with exit code " ++ show n ++
                          ": " ++ trim err))

-- Run one self-contained query.  Starting a process per query is deliberate:
-- the existing SAT abstraction has no close operation, and persistent solver
-- processes would otherwise leak from bsc and bluetcl.
runZ3 :: [String] -> String -> IO SMTResult
runZ3 commands assertion = do
  exe <- findZ3
  let script = unlines (["(set-option :print-success false)"] ++
                        commands ++
                        ["(assert " ++ assertion ++ ")",
                         "(check-sat)",
                         "(exit)"])
  (status, out, err) <- readProcessWithExitCode exe ["-in", "-smt2"] script
  case status of
    ExitFailure n ->
      ioError (userError ("Z3 failed with exit code " ++ show n ++
                          ": " ++ trim err))
    ExitSuccess ->
      case filter (not . null) (map trim (lines out)) of
        ["sat"]     -> return SMTSat
        ["unsat"]   -> return SMTUnsat
        ["unknown"] -> return SMTUnknown
        output -> ioError (userError ("Unexpected Z3 output: " ++ show output ++
                                      if null err then "" else " (" ++ trim err ++ ")"))

trim :: String -> String
trim = dropWhileEnd isSpace . dropWhile isSpace

-- Data.List.dropWhileEnd is not available in every historical compiler used
-- for BSC, so keep this tiny compatibility definition local.
dropWhileEnd :: (a -> Bool) -> [a] -> [a]
dropWhileEnd p = reverse . dropWhile p . reverse
