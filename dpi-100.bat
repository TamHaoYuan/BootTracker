@echo off
setlocal
reg add "HKCU\Control Panel\Desktop" /v LogPixels /t REG_DWORD /d 96 /f >nul 2>&1
reg add "HKCU\Control Panel\Desktop" /v DpiScalingVer /t REG_DWORD /d 0x400 /f >nul 2>&1
echo Done. Logoff or restart required.
