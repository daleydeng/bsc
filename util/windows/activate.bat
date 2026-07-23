@echo off
rem Keep the GHCup and Cabal state project-local so the environment can be reproduced.
set "GHCUP_INSTALL_BASE_PREFIX=%PIXI_PROJECT_ROOT%\.pixi"
set "CABAL_DIR=%PIXI_PROJECT_ROOT%\.pixi\cabal"
set "CABAL_CONFIG=%PIXI_PROJECT_ROOT%\.pixi\cabal\config"
set "MSYSTEM=MINGW64"
set "CHERE_INVOKING=1"

rem Prefer the MinGW tools and MSYS2 shell supplied by this Pixi environment.
rem Include both possible GHCup bin layouts; `pixi.py` also asks GHCup for the
rem authoritative path before invoking any compiler command.
set "PATH=%PIXI_PROJECT_ROOT%\.pixi\ghcup\bin;%PIXI_PROJECT_ROOT%\.pixi\.ghcup\bin;%CONDA_PREFIX%\Library\mingw-w64\bin;%CONDA_PREFIX%\Library\usr\bin;%CONDA_PREFIX%\Library\bin;%PATH%"
set "PKG_CONFIG_PATH=%CONDA_PREFIX%\Library\mingw-w64\lib\pkgconfig;%CONDA_PREFIX%\Library\mingw-w64\share\pkgconfig"
set "SSL_CERT_FILE=%CONDA_PREFIX%\Library\ssl\cacert.pem"
set "GIT_SSL_CAINFO=%CONDA_PREFIX%\Library\ssl\cacert.pem"
set "CURL_CA_BUNDLE=%CONDA_PREFIX%\Library\ssl\cacert.pem"
