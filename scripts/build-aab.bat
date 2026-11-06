@echo off
setlocal
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0build-aab.ps1" %*
exit /b %ERRORLEVEL%
