[CmdletBinding()]
param(
    [Parameter(Mandatory = $true, Position = 0)]
    [ValidateSet("toolchain", "haskell-deps", "doctor", "build", "smoke", "test-z3", "test-rust", "test", "clean", "shell")]
    [string] $Action
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$GhcVersion = "9.6.7"
$GhcWindowsBindist = "https://downloads.haskell.org/~ghc/9.6.7/ghc-9.6.7-x86_64-unknown-mingw32.tar.xz"
$GhcWindowsBindistSha256 = "cf0e736ce4c875de0296426ee575eca177acf6b9b5c1dd4881b9fc79681e1d5f"
$CabalVersion = "3.10.3.0"
$HackageMirror = "https://mirrors.ustc.edu.cn/hackage/"
$HaskellPackages = @("old-time", "regex-compat", "split", "strict-concurrency", "syb")

$Jobs = [Math]::Min([Environment]::ProcessorCount, 16)
if ($env:BSC_JOBS) {
    $Jobs = [int] $env:BSC_JOBS
}
if ($Jobs -lt 1) {
    throw "BSC_JOBS must be a positive integer."
}
$env:BSC_BUILD_JOBS = [string] $Jobs

if (-not $env:PIXI_PROJECT_ROOT -or -not $env:CONDA_PREFIX) {
    throw "This script must run in the Pixi environment. Use 'pixi run $Action'."
}

$Root = $env:PIXI_PROJECT_ROOT

$env:GHCUP_INSTALL_BASE_PREFIX = Join-Path $Root ".pixi"
$env:CABAL_DIR = Join-Path $Root ".pixi\cabal"
$env:CABAL_CONFIG = Join-Path $env:CABAL_DIR "config"
$env:MSYSTEM = "MINGW64"
$env:CHERE_INVOKING = "1"

$GhcupCommand = Get-Command "ghcup.exe" -ErrorAction Stop
$Ghcup = $GhcupCommand.Source
$GhcupBinOutput = & $Ghcup whereis bindir
if ($LASTEXITCODE -ne 0) {
    throw "Unable to determine the project-local GHCup bin directory."
}
$GhcupBin = (($GhcupBinOutput | Out-String).Trim())
$env:BSC_GHCUP_BIN = $GhcupBin

$MingwBin = Join-Path $env:CONDA_PREFIX "Library\mingw-w64\bin"
$MsysBin = Join-Path $env:CONDA_PREFIX "Library\usr\bin"
$CondaLibraryBin = Join-Path $env:CONDA_PREFIX "Library\bin"
$env:Path = (@($GhcupBin, $MingwBin, $MsysBin, $CondaLibraryBin, $env:Path) -join [IO.Path]::PathSeparator)
$env:PKG_CONFIG_PATH = (@(
    (Join-Path $env:CONDA_PREFIX "Library\mingw-w64\lib\pkgconfig"),
    (Join-Path $env:CONDA_PREFIX "Library\mingw-w64\share\pkgconfig")
) -join [IO.Path]::PathSeparator)
$CondaCaBundle = Join-Path $env:CONDA_PREFIX "Library\ssl\cacert.pem"
$env:SSL_CERT_FILE = $CondaCaBundle
$env:GIT_SSL_CAINFO = $CondaCaBundle
$env:CURL_CA_BUNDLE = $CondaCaBundle

function Invoke-Native {
    param(
        [Parameter(Mandatory = $true)] [string] $FilePath,
        [Parameter()] [string[]] $Arguments = @()
    )

    Write-Host "> $FilePath $($Arguments -join ' ')" -ForegroundColor Cyan
    & $FilePath @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "Command failed with exit code ${LASTEXITCODE}: $FilePath $($Arguments -join ' ')"
    }
}

function Get-Sha256 {
    param(
        [Parameter(Mandatory = $true)] [string] $Path
    )

    $Stream = [IO.File]::OpenRead($Path)
    try {
        $Hasher = [Security.Cryptography.SHA256]::Create()
        try {
            $Bytes = $Hasher.ComputeHash($Stream)
            return ([BitConverter]::ToString($Bytes) -replace "-", "").ToLowerInvariant()
        } finally {
            $Hasher.Dispose()
        }
    } finally {
        $Stream.Dispose()
    }
}

function Get-VerifiedDownload {
    param(
        [Parameter(Mandatory = $true)] [string] $Url,
        [Parameter(Mandatory = $true)] [string] $Destination,
        [Parameter(Mandatory = $true)] [string] $Sha256
    )

    $Valid = $false
    if (Test-Path $Destination) {
        $ActualHash = Get-Sha256 $Destination
        $Valid = $ActualHash -eq $Sha256
        if (-not $Valid) {
            Remove-Item -Force $Destination
        }
    }

    if (-not $Valid) {
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Destination) | Out-Null
        $Curl = Join-Path $env:CONDA_PREFIX "Library\bin\curl.exe"
        if (-not (Test-Path $Curl)) {
            throw "Pixi-managed curl was not found at $Curl."
        }
        Invoke-Native $Curl @("--fail", "--location", "--retry", "3", "--output", $Destination, $Url)
        $ActualHash = Get-Sha256 $Destination
        if ($ActualHash -ne $Sha256) {
            Remove-Item -Force $Destination
            throw "SHA256 mismatch for $Url. Expected $Sha256, got $ActualHash."
        }
    }

    return $Destination
}



function Test-GhcupTool {
    param(
        [Parameter(Mandatory = $true)] [string] $Tool,
        [Parameter(Mandatory = $true)] [string] $Version
    )

    $PreviousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        & $Ghcup whereis $Tool $Version 2>$null | Out-Null
        return $LASTEXITCODE -eq 0
    } finally {
        $ErrorActionPreference = $PreviousErrorActionPreference
    }
}

function Initialize-Toolchain {
    New-Item -ItemType Directory -Force -Path $env:CABAL_DIR | Out-Null

    if (-not (Test-GhcupTool "ghc" $GhcVersion)) {
        # conda-forge currently packages GHCup 0.1.18 on Windows. Its compatible
        # metadata predates GHC 9.6.7, so install the official bindist explicitly
        # after checking the SHA256 published in the GHC release's SHA256SUMS.
        $Bindist = Join-Path $Root ".pixi\downloads\ghc-9.6.7-x86_64-unknown-mingw32.tar.xz"
        $Bindist = Get-VerifiedDownload $GhcWindowsBindist $Bindist $GhcWindowsBindistSha256
        # GHCup 0.1.18 mishandles Windows file URIs. Serve the already verified
        # file over loopback for the duration of installation instead.
        $Python = Join-Path $env:CONDA_PREFIX "python.exe"
        if (-not (Test-Path $Python)) {
            throw "Pixi-managed Python was not found at $Python."
        }
        $Listener = New-Object Net.Sockets.TcpListener([Net.IPAddress]::Loopback, 0)
        $Listener.Start()
        $Port = ([Net.IPEndPoint] $Listener.LocalEndpoint).Port
        $Listener.Stop()
        $Server = Start-Process -FilePath $Python -ArgumentList @("-m", "http.server", $Port, "--bind", "127.0.0.1") -WorkingDirectory (Split-Path -Parent $Bindist) -WindowStyle Hidden -PassThru
        try {
            Start-Sleep -Milliseconds 500
            if ($Server.HasExited) {
                throw "The temporary loopback file server exited unexpectedly."
            }
            $BindistUrl = "http://127.0.0.1:$Port/$([Uri]::EscapeDataString((Split-Path -Leaf $Bindist)))"
            Invoke-Native $Ghcup @("--no-verify", "install", "ghc", "--url", $BindistUrl, $GhcVersion, "--set")
        } finally {
            if (-not $Server.HasExited) {
                Stop-Process -Id $Server.Id -Force
            }
        }
    } elseif (((& "ghc.exe" --numeric-version | Out-String).Trim()) -ne $GhcVersion) {
        Invoke-Native $Ghcup @("set", "ghc", $GhcVersion)
    }

    if (-not (Test-GhcupTool "cabal" $CabalVersion)) {
        Invoke-Native $Ghcup @("--cache", "install", "cabal", $CabalVersion)
    } elseif (((& "cabal.exe" --numeric-version | Out-String).Trim()) -ne $CabalVersion) {
        Invoke-Native $Ghcup @("set", "cabal", $CabalVersion)
    }

    Invoke-Native "ghc.exe" @("--version")
    Invoke-Native "cabal.exe" @("--numeric-version")
}

function Set-CabalMirror {
    $Cabal = (Get-Command "cabal.exe" -ErrorAction Stop).Source
    New-Item -ItemType Directory -Force -Path $env:CABAL_DIR | Out-Null

    if (-not (Test-Path $env:CABAL_CONFIG)) {
        Invoke-Native $Cabal @("user-config", "init")
    }

    $Config = Get-Content -Raw $env:CABAL_CONFIG
    $Updated = [regex]::Replace(
        $Config,
        '(?m)^(\s*url:\s*)https?://hackage\.haskell\.org/?\s*$',
        "`${1}$HackageMirror"
    )
    if ($Updated -eq $Config -and $Config -notmatch [regex]::Escape($HackageMirror)) {
        throw "Could not locate the hackage.haskell.org URL in $env:CABAL_CONFIG."
    }
    if ($Updated -ne $Config) {
        Set-Content -Path $env:CABAL_CONFIG -Value $Updated -Encoding ASCII
    }
}

function Install-HaskellDependencies {
    Set-CabalMirror
    $Cabal = (Get-Command "cabal.exe" -ErrorAction Stop).Source
    $GhcPkg = (Get-Command "ghc-pkg.exe" -ErrorAction Stop).Source
    $Missing = @()

    foreach ($Package in $HaskellPackages) {
        $Installed = & $GhcPkg list --simple-output $Package 2>$null
        if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace(($Installed | Out-String))) {
            $Missing += $Package
        }
    }

    if ($Missing.Count -eq 0) {
        Write-Host "Haskell dependencies are already installed." -ForegroundColor Green
        return
    }

    Write-Host "Installing Haskell packages: $($Missing -join ', ')"
    Invoke-Native $Cabal @("update")
    Invoke-Native $Cabal (@("v1-install") + $HaskellPackages)
}

function Invoke-Msys2 {
    param(
        [Parameter()] [AllowEmptyString()] [string] $Command = ""
    )

    $Bash = Join-Path $MsysBin "bash.exe"
    if (-not (Test-Path $Bash)) {
        throw "Pixi-managed MSYS2 bash was not found at $Bash."
    }

    Push-Location $Root
    try {
        if ($Command) {
            Write-Host "> MSYS2: $Command" -ForegroundColor Cyan
            $TempDir = Join-Path $Root ".pixi\tmp"
            New-Item -ItemType Directory -Force -Path $TempDir | Out-Null
            $ScriptName = "pixi-task-$([Guid]::NewGuid().ToString('N')).sh"
            $ScriptPath = Join-Path $TempDir $ScriptName
            $Utf8NoBom = New-Object Text.UTF8Encoding($false)
            $MsysPreamble = @'
export PATH="$(cygpath -u "$CONDA_PREFIX")/Library/bin:$PATH"
export SSL_CERT_FILE="$(cygpath -u "$CONDA_PREFIX")/Library/ssl/cacert.pem"
export GIT_SSL_CAINFO="$SSL_CERT_FILE"
export CURL_CA_BUNDLE="$SSL_CERT_FILE"
'@ + "`n"
            [IO.File]::WriteAllText($ScriptPath, $MsysPreamble + $Command + "`n", $Utf8NoBom)
            try {
                & $Bash --noprofile --norc ".pixi/tmp/$ScriptName"
            } finally {
                Remove-Item -Force -ErrorAction SilentlyContinue $ScriptPath
            }
        } else {
            & $Bash --noprofile --norc -i
        }
        if ($LASTEXITCODE -ne 0) {
            throw "MSYS2 command failed with exit code $LASTEXITCODE."
        }
    } finally {
        Pop-Location
    }
}

function Invoke-CargoTest {
    param(
        [Parameter()] [string[]] $AdditionalArguments = @()
    )

    $Cargo = (Get-Command "cargo.exe" -ErrorAction Stop).Source
    $env:CARGO_TARGET_DIR = Join-Path $Root ".pixi\tmp\cargo-target"
    $Arguments = @(
        "test",
        "--manifest-path", "rust-tests/Cargo.toml",
        "--jobs", [string] $Jobs
    ) + $AdditionalArguments + @(
        "--",
        "--test-threads", [string] $Jobs
    )
    Invoke-Native $Cargo $Arguments
}


switch ($Action) {
    "toolchain" {
        Initialize-Toolchain
    }
    "haskell-deps" {
        Install-HaskellDependencies
    }


    "doctor" {
        Invoke-Msys2 @'
set -u
failed=0
for tool in bash make git diff gcc g++ perl pkg-config tclsh iverilog ghc ghc-pkg cabal rustc cargo z3; do
    if command -v "$tool" >/dev/null 2>&1; then
        printf '%-12s %s\n' "$tool" "$(command -v "$tool")"
    else
        printf '%-12s MISSING\n' "$tool"
        failed=1
    fi
done
printf '%-12s %s\n' OSTYPE "$(./platform.sh ostype)"
printf '%-12s %s\n' MACHTYPE "$(./platform.sh machtype)"
printf '%-12s %s\n' BUILD_JOBS "$BSC_BUILD_JOBS"
ghc --version 2>/dev/null || true
cabal --numeric-version 2>/dev/null || true
rustc --version 2>/dev/null || true
cargo --version 2>/dev/null || true
z3 -version 2>/dev/null || true
exit "$failed"
'@
    }
    "build" {
        $GhcTemp = Join-Path $Root ".pixi\tmp\ghc"
        New-Item -ItemType Directory -Force -Path $GhcTemp | Out-Null
        $env:TEMP = $GhcTemp
        $env:TMP = $GhcTemp
        Write-Host "Building with $Jobs parallel jobs (set BSC_JOBS to override)." -ForegroundColor Green
        Invoke-Msys2 "make -j$Jobs GHCJOBS=$Jobs install-src"
    }
    "smoke" {
        Invoke-Msys2 "make check-smoke"
    }
    "test-z3" {
        Invoke-CargoTest @("--test", "scheduler_sat")
    }
    "test-rust" {
        Invoke-CargoTest
    }
    "test" {
        Invoke-CargoTest
    }
    "clean" {
        Invoke-Msys2 "make full_clean"
    }
    "shell" {
        Invoke-Msys2
    }
}
