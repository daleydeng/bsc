# BSC native Windows build recipes. Pixi supplies `just` and all other tools.

ps := "powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File util/windows/pixi.ps1"

# List available recipes.
default:
    @just --list --unsorted

# Install and select the project-local GHC and Cabal toolchain.
toolchain:
    {{ps}} toolchain

# Install BSC's Haskell package dependencies using the configured mirror.
haskell-deps: toolchain
    {{ps}} haskell-deps

# Prepare the complete native Windows build environment.
bootstrap: haskell-deps

# Report the exact tools and platform seen by the build.
doctor: bootstrap
    {{ps}} doctor

# Build and install BSC into ./inst.
build: haskell-deps
    {{ps}} build

# Build BSC and run the upstream smoke test.
smoke: build
    {{ps}} smoke

# Remove the upstream build and installation directories.
clean:
    {{ps}} clean

# Enter the Pixi-managed MSYS2 shell used by the build.
msys2-shell:
    {{ps}} shell
