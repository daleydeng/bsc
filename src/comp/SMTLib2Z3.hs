{-# LANGUAGE ForeignFunctionInterface #-}

module SMTLib2Z3(
       SMTResult(..),
       checkVersion,
       runZ3
) where

import qualified Data.ByteString as BS
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE
import Data.Text.Encoding.Error(lenientDecode)
import Data.Word(Word8)
import Foreign.C.Types(CInt(..), CSize(..))
import Foreign.Marshal.Alloc(allocaBytes)
import Foreign.Ptr(Ptr, castPtr, nullPtr)

-- Keep the solver result distinct from Bool: "unknown" must never be
-- mistaken for either a proof or a counterexample.
data SMTResult = SMTSat | SMTUnsat | SMTUnknown
                 deriving (Eq, Ord, Show)

foreign import ccall safe "bsc_z3_check_smtlib2"
  c_bsc_z3_check_smtlib2 :: Ptr Word8 -> CSize -> Ptr Word8 -> CSize -> IO CInt

foreign import ccall unsafe "bsc_z3_version"
  c_bsc_z3_version :: Ptr Word8 -> CSize -> IO CSize

checkVersion :: IO String
checkVersion = do
  versionLength <- c_bsc_z3_version nullPtr 0
  let bufferLength = fromIntegral versionLength + 1
  if versionLength == 0
    then ioError (userError "Rust Z3 bridge could not query the linked Z3 version")
    else allocaBytes bufferLength $ \versionPtr -> do
      reportedLength <- c_bsc_z3_version versionPtr (fromIntegral bufferLength)
      if reportedLength /= versionLength
        then ioError (userError "Rust Z3 bridge returned an inconsistent Z3 version length")
        else do
          versionBytes <- BS.packCStringLen
                            (castPtr versionPtr, fromIntegral versionLength)
          return ("Z3 " ++ T.unpack (TE.decodeUtf8With lenientDecode versionBytes))

-- The bridge creates a fresh in-process Z3 solver for each query.  This keeps
-- the established isolation semantics without paying Windows process-startup
-- cost for every scheduler and AOpt check.
runZ3 :: [String] -> String -> IO SMTResult
runZ3 commands assertion = do
  let script = unlines (["(set-option :print-success false)"] ++
                        commands ++
                        ["(assert " ++ assertion ++ ")"])
      input = TE.encodeUtf8 (T.pack script)
      errorCapacity = 16384
  allocaBytes errorCapacity $ \errorPtr ->
    BS.useAsCStringLen input $ \(inputPtr, inputLength) -> do
      result <- c_bsc_z3_check_smtlib2
                  (castPtr inputPtr)
                  (fromIntegral inputLength)
                  errorPtr
                  (fromIntegral errorCapacity)
      case result of
        0 -> return SMTSat
        1 -> return SMTUnsat
        2 -> return SMTUnknown
        3 -> do
          errorBytes <- BS.packCString (castPtr errorPtr)
          let detail = T.unpack (TE.decodeUtf8With lenientDecode errorBytes)
          ioError (userError ("Rust Z3 bridge failed" ++
                              if null detail then "" else ": " ++ detail))
        code -> ioError (userError ("Rust Z3 bridge returned invalid result " ++
                                    show code))
