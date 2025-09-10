@echo off
REM Debug plugin build and install script for Windows with pre-built Skia
REM Usage: bin\debug_plugin.bat

REM Allow skipping the build phase when called from preflight script
if /I "%1"=="install-only" goto install_only
if /I "%SKIP_BUILD%"=="1" goto install_only

echo === Ensuring Rust is available in PATH ===
set PATH=%USERPROFILE%\.cargo\bin;%SystemRoot%\system32;%SystemRoot%;%SystemRoot%\System32\WindowsPowerShell\v1.0;%PATH%

echo === Configuring Rust for MSVC target ===
rustup target add x86_64-pc-windows-msvc

echo === Cleaning previous builds ===
cargo clean

echo === Configuring Skia to use pre-built binaries ===
REM Always force Skia to download pre-built binaries instead of compiling
set FORCE_SKIA_BINARIES_DOWNLOAD=1
set SKIA_BINARIES_URL=https://github.com/rust-skia/skia-binaries/releases/download/0.84.0/
set SKIA_BINARIES_KEY=skia-binaries-0.84.0

REM Force MSVC target detection
set TARGET=x86_64-pc-windows-msvc
set CARGO_CFG_TARGET_ENV=msvc

REM Set BUILD_DATE for compilation
set BUILD_DATE=%date:~-4,4%%date:~-10,2%%date:~-7,2%

echo === Building VST3 plugin with vizia GUI ===
echo Features: api5500, buttercomp2, transformer, gui (vizia with pre-built Skia)
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
