module System.Posix.Files(
       FileStatus,
       FileMode,
       fileMode,
       unionFileModes,
       ownerExecuteMode,
       groupExecuteMode,
       setFileMode,
       getFileStatus,
       fileAccess,
       fileExist,
       modificationTime
) where

import Data.Bits((.|.))
import Data.Time.Clock(UTCTime)
import Data.Word(Word32)
import System.Directory(doesFileExist, getModificationTime)

data FileStatus = FileStatus {
    modificationTime :: UTCTime,
    fileMode :: FileMode
  }

newtype FileMode = FileMode Word32

unionFileModes :: FileMode -> FileMode -> FileMode
unionFileModes (FileMode a) (FileMode b) = FileMode (a .|. b)

ownerExecuteMode, groupExecuteMode :: FileMode
ownerExecuteMode = FileMode 0x40
groupExecuteMode = FileMode 0x08

getFileStatus :: FilePath -> IO FileStatus
getFileStatus path = do
  modified <- getModificationTime path
  return FileStatus { modificationTime = modified, fileMode = FileMode 0 }

-- Windows does not use POSIX executable mode bits.  Keeping this operation a
-- no-op preserves the caller's intent without changing ACLs.
setFileMode :: FilePath -> FileMode -> IO ()
setFileMode _ _ = return ()

fileAccess :: FilePath -> Bool -> Bool -> Bool -> IO Bool
fileAccess path _ _ _ = doesFileExist path

fileExist :: FilePath -> IO Bool
fileExist = doesFileExist
