@echo off
setlocal EnableExtensions

echo === Bus Channel Strip: Simplified Build Script ===

REM Ensure Rust & Cargo are available
set PATH=%USERPROFILE%\.cargo\bin;%PATH%

echo.
echo [1/3] Checking nightly Rust toolchain
rustup toolchain list | findstr "nightly" >nul
if errorlevel 1 (
  echo   - Installing nightly Rust toolchain...
  rustup toolchain install nightly
) else (
  echo   - Nightly Rust toolchain found
)

echo.
echo [2/3] Building xtask binary
cargo +nightly build --package xtask --quiet
if errorlevel 1 (
  echo   ! xtask build failed
  exit /b 1
) else (
  echo   - xtask build successful
)

echo.
echo [3/3] Building VST plugin with GUI
echo Features: api5500, buttercomp2, pultec, transformer, punch, gui
echo.
set "FORCE_SKIA_BINARIES_DOWNLOAD=1"
set "RUST_BACKTRACE=1"
set "LLVM_HOME=C:\Program Files\LLVM"
set "LIBCLANG_PATH=C:\Program Files\LLVM\bin"
cargo +nightly run --package xtask -- bundle bus_channel_strip --release --features api5500,buttercomp2,pultec,transformer,punch,gui

if errorlevel 1 (
  echo.
  echo Build failed. See errors above.
  exit /b 1
) else (
  echo.
  echo ✓ Build completed successfully.
  if exist "target\bundled\Bus-Channel-Strip.vst3" (
    echo   - VST3: target\bundled\Bus-Channel-Strip.vst3\
  )
  if exist "target\bundled\Bus-Channel-Strip.clap" (
    echo   - CLAP: target\bundled\Bus-Channel-Strip.clap
  )
  echo.
  echo === VST plugin ready for testing ===
)

endlocal
exit /b 0