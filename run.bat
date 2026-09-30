@echo off
cd /d "%~dp0"
if not exist "target\release\api.exe" (
    echo API binary not found. Run build.bat first.
    exit /b 1
)
"target\release\api.exe" %*
