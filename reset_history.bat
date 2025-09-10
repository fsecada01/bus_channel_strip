@echo off
echo =====================================================
echo Git History Reset - Fresh Start
echo =====================================================
echo.
echo This will create a completely new git history.
echo Your current work will be preserved.
echo.
pause

echo.
echo [1/7] Creating backup branch...
git branch backup-before-reset
if %errorlevel% neq 0 (
    echo Error: Failed to create backup branch
    exit /b 1
)
echo ✓ Backup created: backup-before-reset

echo.
echo [2/7] Checking git status...
git status --porcelain > nul
echo ✓ Git status checked

echo.
echo [3/7] Creating fresh orphan branch...
git checkout --orphan fresh-main
if %errorlevel% neq 0 (
    echo Error: Failed to create orphan branch
    exit /b 1
)
echo ✓ Created fresh-main branch

echo.
echo [4/7] Adding all files...
git add .
if %errorlevel% neq 0 (
    echo Error: Failed to add files
    exit /b 1
)
echo ✓ All files staged

echo.
echo [5/7] Creating initial commit...
git commit -m "Initial commit: Bus Channel Strip VST Plugin

- Multi-module bus channel strip with API5500 EQ, ButterComp2, Pultec EQ, Dynamic EQ, and Transformer
- Built with NIH-Plug framework and vizia GUI
- Cross-platform CI/CD pipeline with pre-built Skia binaries  
- Professional parameter set (~75 parameters) with module reordering
- Working VST3 and CLAP bundle creation for Windows, Linux, and macOS
- All core DSP modules implemented and functional

🤖 Generated with Claude Code
Co-Authored-By: Claude <noreply@anthropic.com>" -S
if %errorlevel% neq 0 (
    echo Error: Failed to create commit
    exit /b 1
)
echo ✓ Initial commit created

echo.
echo [6/7] Renaming branch to main...
git branch -M main
if %errorlevel% neq 0 (
    echo Error: Failed to rename branch
    exit /b 1
)
echo ✓ Branch renamed to main

echo.
echo [7/7] Force pushing to origin...
echo WARNING: This will overwrite the remote repository history!
echo Press Ctrl+C to cancel, or
pause

git push -f origin main
if %errorlevel% neq 0 (
    echo Error: Failed to push to origin
    echo Your local changes are safe in the backup-before-reset branch
    exit /b 1
)

echo.
echo =====================================================
echo ✓ SUCCESS: Git history reset complete!
echo =====================================================
echo.
echo - New clean history with 1 commit
echo - Old history preserved in: backup-before-reset
echo - Remote repository updated
echo.
echo To verify: git log --oneline
echo.
pause