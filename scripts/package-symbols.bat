@echo off
setlocal
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0package-symbols.ps1" %*
exit /b %ERRORLEVEL%
