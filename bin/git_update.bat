@echo off
echo Committing changes to the Git Repo

set BRANCH=%1
set ARG2=%2
set ARG3=%3

REM If only two args, assume -m flag with memo content
if "%ARG3%"=="" (
    if not "%ARG2%"=="" (
        set "COMMIT_MSG=%ARG2%"
        set "USE_MEMO_ARG=1"
        goto :commit
    )
) else (
    REM Three args: check ARG2 for flag
    if "%ARG2%"=="-m" (
        set "COMMIT_MSG=%ARG3%"
        set "USE_MEMO_ARG=1"
        goto :commit
    ) else if "%ARG2%"=="-F" (
        if not exist "%ARG3%" (
            echo Error: File "%ARG3%" does not exist
            exit /b 1
        )
        set "COMMIT_FILE=%ARG3%"
        set "USE_FILE=1"
        goto :commit
    ) else (
        REM ARG2 is not a recognized flag, treat as memo (original behavior)
        set "MEMO=%ARG2%"
    )
)

REM Prompt for missing values
IF "%BRANCH%"=="" (
    set /p BRANCH="What is the branch to which you are committing code?  master or deploy?  " || set "BRANCH=deploy"
)

REM If no flag was used and we don't have a memo, prompt for it
if not defined USE_MEMO_ARG if not defined USE_FILE (
    IF "%MEMO%"=="" (
        set /p MEMO="Enter memo for Git Commit: "
    )
)

:commit
git rm -r --cached .
git status
git add --chmod=+x -- *.sh
git add --all .

REM Commit based on flag used
if defined USE_FILE (
    type "%COMMIT_FILE%" | git commit --file=- -S
) else if defined USE_MEMO_ARG (
    git commit -m "%COMMIT_MSG%" -S
) else (
    git commit -m %MEMO% -S
)

git push origin "%BRANCH%"
@echo on