module System.Posix.Signals(
       Signal,
       Handler(..),
       SignalSet,
       sigINT,
       installHandler
) where

-- The compiler only installs a Ctrl-C callback and ignores the previous
-- handler.  Tcl/Windows already handles console interruption, so expose the
-- POSIX-shaped API without introducing a Unix package dependency.
data Signal = SignalInt

data Handler = Default | Ignore | Catch (IO ())

data SignalSet = SignalSet

sigINT :: Signal
sigINT = SignalInt

installHandler :: Signal -> Handler -> Maybe SignalSet -> IO Handler
installHandler _ handler _ = return handler
