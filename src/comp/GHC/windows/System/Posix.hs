module System.Posix(getProcessID) where

import System.Win32.Process(getCurrentProcessId)

getProcessID :: IO Integer
getProcessID = fromIntegral <$> getCurrentProcessId
