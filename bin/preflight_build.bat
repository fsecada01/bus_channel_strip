@echo off
setlocal EnableExtensions EnableDelayedExpansion

echo === Bus Channel Strip: Windows Preflight + Build ===

REM Ensure Rust & Cargo are available
set PATH=%USERPROFILE%\.cargo\bin;%SystemRoot%\system32;%SystemRoot%;%SystemRoot%\System32\WindowsPowerShell\v1.0;%PATH%

REM Try to import MSVC environment (vcvars64) if available
set "VCVARS=%ProgramFiles(x86)%\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
if exist "%VCVARS%" (
  echo   - Importing MSVC environment from vcvars64.bat
  call "%VCVARS%" >nul 2>&1
) else (
  echo   - vcvars64.bat not found at expected Build Tools path. Continuing.
)

REM Default: purge partial Skia extractions unless explicitly disabled
if not defined SKIA_PURGE set "SKIA_PURGE=1"

REM Detect user drive (used for symlink test on the same volume as the Cargo registry)
for /f "delims=" %%I in ("%USERPROFILE%") do set "USERDRV=%%~dI"
if "%USERDRV%"=="" set "USERDRV=%SystemDrive%"

echo.
echo [1/6] Checking Windows Developer Mode (symlink support)
for /f "tokens=3" %%A in ('reg query "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock" /v AllowDevelopmentWithoutDevLicense 2^>nul ^| find /I "AllowDevelopmentWithoutDevLicense"') do set "DEVREG=%%A"
if not defined DEVREG (
  echo   - Developer Mode registry key not found.
) else (
  echo   - Developer Mode registry value: %DEVREG%
)

echo   - Testing symlink creation on %USERDRV% ...
set "_PF_TMP=%USERDRV%\_symlink_test_%RANDOM%_%RANDOM%"
if exist "%_PF_TMP%" rmdir /s /q "%_PF_TMP%" >nul 2>&1
mkdir "%_PF_TMP%" >nul 2>&1
copy /y "%SystemRoot%\win.ini" "%_PF_TMP%\target_win.ini" >nul 2>&1
mklink "%_PF_TMP%\link.ini" "%_PF_TMP%\target_win.ini" >nul 2>&1
if errorlevel 1 (
  echo   ! Symlink creation FAILED.
  echo     Enable Developer Mode ^(Settings ^> Privacy ^& Security ^> For Developers^),
  echo     or run this script in an elevated ^(Administrator^) Command Prompt.
  echo     You can also grant your user "Create symbolic links" in Local Security Policy.
  set PF_SYMLINK_OK=0
) else (
  echo   - Symlink creation OK.
  set PF_SYMLINK_OK=1
)
if exist "%_PF_TMP%\link.ini" del /f /q "%_PF_TMP%\link.ini" >nul 2>&1
if exist "%_PF_TMP%\target_win.ini" del /f /q "%_PF_TMP%\target_win.ini" >nul 2>&1
if exist "%_PF_TMP%" rmdir /s /q "%_PF_TMP%" >nul 2>&1

echo.
echo [2/6] Checking LLVM / clang-cl required by bindgen
set "LLVM_HOME=C:\Program Files\LLVM"
if exist "%LLVM_HOME%\bin\clang-cl.exe" (
  echo   - Found clang-cl at "%LLVM_HOME%\bin\clang-cl.exe"
) else (
  where clang-cl >nul 2>&1
  if errorlevel 1 (
    echo   ! clang-cl not found.
    echo     Install LLVM 17 ^(recommended^) and try again. Examples:
    echo       choco install llvm --version=17.0.6 -y
    echo       winget install LLVM.LLVM
    set PF_LLVM_OK=0
  ) else (
    for /f "delims=" %%I in ('where clang-cl') do set "CLANG_BIN=%%~dpI"
    echo   - Found clang-cl on PATH at "%CLANG_BIN%clang-cl.exe"
    set "LLVM_HOME=%CLANG_BIN%.."
    set PF_LLVM_OK=1
  )
)
if not defined PF_LLVM_OK set PF_LLVM_OK=1
set "LIBCLANG_PATH=%LLVM_HOME%\bin"
set "CC=clang-cl"
set "CXX=clang-cl"
REM Help bindgen parse MSVC STL headers correctly
set "BINDGEN_EXTRA_CLANG_ARGS=--target=x86_64-pc-windows-msvc -std=c++17 -fms-compatibility -fms-compatibility-version=19 -fdelayed-template-parsing -march=native -I"%LLVM_HOME%\include""
REM Also set CLANG_ARGS for skia-bindings
set "CLANG_ARGS=--target=x86_64-pc-windows-msvc -std=c++17 -fms-compatibility -fms-compatibility-version=19 -fdelayed-template-parsing -march=native -I"%LLVM_HOME%\include""

echo.
echo [2b] Checking clang-cl version and preferring LLVM 16
if exist "%LLVM_HOME%\bin\clang-cl.exe" (
  "%LLVM_HOME%\bin\clang-cl.exe" --version | findstr /R /C:"version [0-9][0-9]*" >nul 2>&1
  if errorlevel 1 (
    echo   ! Could not determine clang-cl version.
  ) else (
    "%LLVM_HOME%\bin\clang-cl.exe" --version | findstr /R /C:"version \(1[7-9]\|2[0-9]\)." >nul 2>&1
    if not errorlevel 1 (
      for /f "tokens=3" %%V in ('"%LLVM_HOME%\bin\clang-cl.exe" --version 2^>nul') do (
        echo   ! Detected LLVM %%V. Trying to locate LLVM 16 on PATH...
        goto :try_find_llvm16
      )
      :try_find_llvm16
      for /f "delims=" %%P in ('where clang-cl 2^>nul') do (
        "%%P" --version | findstr /C:"version 16." >nul 2>&1
        if not errorlevel 1 (
          set "LLVM_HOME=%%~dpP.."
          set "LIBCLANG_PATH=%%~dpP"
          echo     -> Using LLVM at "%%~dpP"
          goto :llvm16_selected
        )
      )
      echo   ! LLVM 16 not found on PATH. Consider installing: choco install llvm --version=16.0.6 -y
    )
  )
) else (
  echo   ! clang-cl not found at expected path.
)
:llvm16_selected

echo.
echo [2a] Locating MSVC and Windows SDK include paths
set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
if exist "%VSWHERE%" (
  for /f "usebackq delims=" %%P in (`"%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2^>nul`) do (
    set "VSINSTALL=%%P"
  )
  if defined VSINSTALL (
    for /f "delims=" %%D in ('dir /b /ad "!VSINSTALL!\VC\Tools\MSVC" 2^>nul ^| sort') do set "_MSVC_VER=%%D"
    if defined _MSVC_VER (
      set "MSVC_INCLUDE=!VSINSTALL!\VC\Tools\MSVC\!_MSVC_VER!\include"
      if exist "!MSVC_INCLUDE!" (
        echo   - MSVC include: "!MSVC_INCLUDE!"
        set BINDGEN_EXTRA_CLANG_ARGS=!BINDGEN_EXTRA_CLANG_ARGS! -I"!MSVC_INCLUDE!"
        set CLANG_ARGS=!CLANG_ARGS! -I"!MSVC_INCLUDE!"
        set PF_MSVC_OK=1
      ) else (
        echo   ! Could not locate MSVC include directory.
        set PF_MSVC_OK=0
      )
    ) else (
      echo   ! Could not find MSVC version directory.
      set PF_MSVC_OK=0
    )
  ) else (
    echo   ! Visual Studio Build Tools not found by vswhere.
    set PF_MSVC_OK=0
  )
) else (
  echo   ! vswhere.exe not found.
  echo     Install Visual Studio Build Tools 2022 with C++ workload and Windows SDK.
  echo     Download: https://aka.ms/vs/17/release/vs_BuildTools.exe
  echo     Required components include:
  echo       - Microsoft.VisualStudio.Component.VC.Tools.x86.x64
  echo       - Microsoft.VisualStudio.Component.Windows10SDK.19041 ^(or newer^)
  echo     After install, re-open an elevated Command Prompt and re-run this script.
  set PF_MSVC_OK=0
)

set "WIN10_INC=%ProgramFiles(x86)%\Windows Kits\10\Include"
if exist "%WIN10_INC%" (
  REM Get any Windows SDK version  
  for /f "tokens=* delims=" %%D in ('dir /b /ad "%WIN10_INC%" 2^>nul') do (
    echo %%D | findstr /R "^10\." >nul
    if not errorlevel 1 (
      set "_WINSDK_VER=%%D"
    )
  )
  if defined _WINSDK_VER (
    echo   - Windows SDK version: !_WINSDK_VER!
    set "SDK_UCRT=!WIN10_INC!\!_WINSDK_VER!\ucrt"
    set "SDK_SHARED=!WIN10_INC!\!_WINSDK_VER!\shared"
    set "SDK_UM=!WIN10_INC!\!_WINSDK_VER!\um"
    set "SDK_WINRT=!WIN10_INC!\!_WINSDK_VER!\winrt"
    for %%I in ("!SDK_UCRT!" "!SDK_SHARED!" "!SDK_UM!" "!SDK_WINRT!") do (
      if exist "%%~I" (
        echo   - Windows SDK include: %%~I
        set BINDGEN_EXTRA_CLANG_ARGS=!BINDGEN_EXTRA_CLANG_ARGS! -I"%%~I"
        set CLANG_ARGS=!CLANG_ARGS! -I"%%~I"
      )
    )
    set PF_SDK_OK=1
  ) else (
    echo   ! Could not detect Windows SDK version.
    set PF_SDK_OK=0
  )
) else (
  echo   ! Windows 10/11 SDK include directory not found.
  set PF_SDK_OK=0
)

echo.
echo [3/6] Checking Ninja (used by Skia build tooling)
where ninja >nul 2>&1
if errorlevel 1 (
  if exist "%ProgramData%\chocolatey\bin\ninja.exe" (
    echo   - Found Ninja at "%ProgramData%\chocolatey\bin\ninja.exe"
    set "SKIA_NINJA_COMMAND=%ProgramData%\chocolatey\bin\ninja.exe"
    set PF_NINJA_OK=1
  ) else (
    echo   ! ninja not found.
    echo     Install Ninja and try again. Examples:
    echo       choco install ninja -y
    echo       winget install Ninja-build.Ninja
    set PF_NINJA_OK=0
  )
) else (
  for /f "delims=" %%I in ('where ninja 2^>nul') do (
    if not defined SKIA_NINJA_COMMAND set "SKIA_NINJA_COMMAND=%%I"
  )
  if not defined SKIA_NINJA_COMMAND (
    echo   ! where ninja succeeded but path not captured; defaulting to "ninja" on PATH
    set "SKIA_NINJA_COMMAND=ninja"
  )
  echo   - Found Ninja at "%SKIA_NINJA_COMMAND%"
  set PF_NINJA_OK=1
)

echo.
echo [4/6] Configuring Skia to use prebuilt binaries when possible
set FORCE_SKIA_BINARIES_DOWNLOAD=1
REM Optionally point to a local cache directory (uncomment and adjust):
REM set SKIA_BINARIES_URL=file:///C:/dev/skia-binaries-0.84.0/
REM set SKIA_BINARIES_KEY=skia-binaries-0.84.0

REM Purge partial Skia extractions that may have failed earlier
echo   - Purging partial Skia extraction in Cargo registry...
for /d %%D in ("%USERPROFILE%\.cargo\registry\src\index.crates.io-*\skia-bindings-0.84.0\skia-*") do (
  REM echo     Removing: %%D
  REM rmdir /s /q "%%D" >nul 2>&1
)

echo.
echo [5/6] Setting BUILD_DATE for version suffix
for /f %%I in ('powershell -NoProfile -Command "(Get-Date).ToString('yyyyMMdd')"') do set BUILD_DATE=%%I
if not defined BUILD_DATE set BUILD_DATE=dev
echo   - BUILD_DATE = %BUILD_DATE%

echo.
echo [6/6] Summary
echo   - Symlink OK:        %PF_SYMLINK_OK%
echo   - LLVM/clang-cl OK:  %PF_LLVM_OK%
echo   - Ninja OK:          %PF_NINJA_OK%
echo   - MSVC Includes OK:  %PF_MSVC_OK%
echo   - WinSDK Includes OK:%PF_SDK_OK%

if "%PF_SYMLINK_OK%"=="0" (
  echo.
  echo Build will likely fail due to symlink restrictions.
  echo Please enable Developer Mode or run elevated and re-run this script.
  goto :EOF
)
if "%PF_LLVM_OK%"=="0" (
  echo.
  echo Build will likely fail due to missing clang-cl / LLVM.
  goto :EOF
)
if "%PF_NINJA_OK%"=="0" (
  echo.
  echo Build may fail due to missing Ninja.
  goto :EOF
)
if "%PF_MSVC_OK%"=="0" (
  echo.
  echo Build will likely fail due to missing MSVC headers.
  echo Please install Visual Studio Build Tools 2022 with C++ workload.
  goto :EOF
)
if "%PF_SDK_OK%"=="0" (
  echo.
  echo Build will likely fail due to missing Windows 10/11 SDK headers.
  echo Please install the Windows SDK component in VS Build Tools and try again.
  goto :EOF
)

echo.
echo === Running bundle build ===
echo Features: api5500, buttercomp2, transformer, gui
echo.
echo Using minimal environment for successful build with prebuilt Skia binaries...
REM Clear potentially problematic environment variables
set "BINDGEN_EXTRA_CLANG_ARGS="
set "CLANG_ARGS="
set "CC="
set "CXX="
set "LIBCLANG_PATH="
REM Set only essential environment variables
set "FORCE_SKIA_BINARIES_DOWNLOAD=1"
set "RUST_BACKTRACE=1"
cargo +nightly run --package xtask -- bundle bus_channel_strip --release --features api5500,buttercomp2,transformer,gui
if errorlevel 1 (
  echo.
  echo Build failed. See errors above.
  exit /b 1
) else (
  echo.
  echo ✓ Build completed.
  if exist "target\bundled\Bus-Channel-Strip.vst3" (
    echo   - VST3: target\bundled\Bus-Channel-Strip.vst3
  ) else (
    echo   - VST3 not found in target\bundled\
  )
  if exist "target\bundled\Bus-Channel-Strip.clap" (
    echo   - CLAP: target\bundled\Bus-Channel-Strip.clap
  ) else (
    echo   - CLAP not found in target\bundled\
  )
  echo.
  echo === Installing plugin bundles (install-only) ===
  call bin\debug_plugin.bat install-only
)

endlocal
exit /b 0