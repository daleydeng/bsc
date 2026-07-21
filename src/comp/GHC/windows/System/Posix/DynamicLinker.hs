module System.Posix.DynamicLinker(
       DL,
       RTLDFlags(..),
       dlopen,
       dlsym,
       dlclose
) where

import Foreign.Ptr(FunPtr, castPtrToFunPtr)
import System.Win32.DLL(loadLibrary, freeLibrary, getProcAddress)
import System.Win32.Types(HMODULE)

data DL = DL HMODULE

data RTLDFlags = RTLD_LAZY | RTLD_NOW | RTLD_GLOBAL | RTLD_LOCAL
                 deriving (Eq, Show)

dlopen :: FilePath -> [RTLDFlags] -> IO DL
dlopen path _ = DL <$> loadLibrary path

dlsym :: DL -> String -> IO (FunPtr a)
dlsym (DL handle) symbol = castPtrToFunPtr <$> getProcAddress handle symbol

dlclose :: DL -> IO ()
dlclose (DL handle) = freeLibrary handle
