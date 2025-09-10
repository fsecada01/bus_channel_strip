@echo off
REM Simple debug plugin build and install script
REM Usage: bin\debug_plugin_simple.bat

REM Allow skipping the build phase when called from preflight script
if /I "%1"=="install-only" goto install_only
if /I "%SKIP_BUILD%"=="1" goto install_only

echo === Simple VST Plugin Build and Install ===

echo === Ensuring nightly Rust toolchain ===
rustup toolchain list | findstr "nightly" >nul
if errorlevel 1 (
  echo Installing nightly Rust toolchain...
  rustup toolchain install nightly
) else (
  echo Nightly Rust toolchain found
)

echo === Building xtask ===
cargo +nightly build --package xtask --quiet
if errorlevel 1 (
  echo Error: xtask build failed
  exit /b 1
)

echo === Building VST3 plugin with GUI ===
echo Features: api5500, buttercomp2, transformer, gui
REM Use minimal environment for reliable build
set "FORCE_SKIA_BINARIES_DOWNLOAD=1"
set "RUST_BACKTRACE=1"
cargo +nightly run --package xtask -- bundle bus_channel_strip --release --features api5500,buttercomp2,transformer,gui

if %errorlevel% neq 0 (
    echo Error: Build failed
    exit /b 1
) else (
    echo ✓ Build completed successfully!
    echo Checking bundled files...
    if exist "target\bundled\Bus-Channel-Strip.vst3" (
        echo ✓ VST3 plugin found: target\bundled\Bus-Channel-Strip.vst3
        dir "target\bundled\Bus-Channel-Strip.vst3" | find "Directory" > nul && echo   - VST3 bundle structure created
    ) else (
        echo ✗ VST3 plugin not found
        exit /b 1
    )
    if exist "target\bundled\Bus-Channel-Strip.clap" (
        echo ✓ CLAP plugin found: target\bundled\Bus-Channel-Strip.clap
        for %%I in ("target\bundled\Bus-Channel-Strip.clap") do echo   - CLAP size: %%~zI bytes
    ) else (
        echo ✗ CLAP plugin not found
    )
)

goto do_install

:install_only
echo === Skipping build: install-only mode ===

:do_install
echo === Removing old plugin ===
if exist "C:\Program Files\Common Files\VST3\Bus-Channel-Strip.vst3" (
    rmdir /s /q "C:\Program Files\Common Files\VST3\Bus-Channel-Strip.vst3"
    echo Removed old plugin
) else (
    echo No existing plugin found
)

echo === Installing new plugin ===
echo Copying VST3 plugin to system directory...
xcopy /e /i /y "target\bundled\Bus-Channel-Strip.vst3" "C:\Program Files\Common Files\VST3\Bus-Channel-Strip.vst3"

if %errorlevel% neq 0 (
    echo Error: Plugin installation failed - check if running as administrator
    echo Trying user-local VST3 directory instead...
    if not exist "%USERPROFILE%\Documents\VST3" mkdir "%USERPROFILE%\Documents\VST3"
    xcopy /e /i /y "target\bundled\Bus-Channel-Strip.vst3" "%USERPROFILE%\Documents\VST3\Bus-Channel-Strip.vst3"
    if %errorlevel% neq 0 (
        echo Error: Plugin installation failed in both system and user directories
        exit /b 1
    ) else (
        echo ✓ Plugin installed successfully to user directory!
        echo Location: %USERPROFILE%\Documents\VST3\Bus-Channel-Strip.vst3
    )
) else (
    echo ✓ Plugin installed successfully to system directory!
    echo Location: C:\Program Files\Common Files\VST3\Bus-Channel-Strip.vst3
)

echo === Plugin ready for testing in DAWs ===
echo The plugin should now appear in your DAW's plugin scanner
pause