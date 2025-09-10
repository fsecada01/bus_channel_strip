@echo off
echo Committing changes to the Git Repo


set BRANCH=%1
set MEMO=%2

IF %BRANCH%=="" (
    set /p BRANCH="What is the branch to which you are committing code?  master or deploy?  " || set "branch=deploy"
)
IF %MEMO%=="" (
    set /p MEMO="Enter memo for Git Commit: "
)

git rm -r --cached .
git status
git add --chmod=+x -- *.sh
git add --chmod=+x -- tests\scripts\*.py
for /F "delims=" %%f in ('dir /q /b *test*.py') do git add --chmod=+x %%f
git add --all .
git commit -m %MEMO% -S
git push origin "%BRANCH%"
rem git push origin flask_makeover
@echo on