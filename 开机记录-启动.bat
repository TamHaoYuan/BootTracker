@echo off
rem 开机时间记录 - 桌面应用启动脚本
rem 将此 .bat 文件（或快捷方式）放入 shell:startup 文件夹即可开机自启

set SCRIPT_DIR=%~dp0
set LAUNCHER=%SCRIPT_DIR%boot-tracker.py

if not exist "%LAUNCHER%" (
    echo [错误] 找不到 boot-tracker.py，请确认文件完整
    pause
    exit /b 1
)

rem 优先使用 pythonw（无控制台窗口），回退到 python
where pythonw >nul 2>&1
if %errorlevel%==0 (
    start "" pythonw "%LAUNCHER%"
) else (
    rem 尝试常见安装路径
    if exist "C:\Python313\pythonw.exe" (
        start "" "C:\Python313\pythonw.exe" "%LAUNCHER%"
    ) else if exist "C:\Python312\pythonw.exe" (
        start "" "C:\Python312\pythonw.exe" "%LAUNCHER%"
    ) else (
        rem 最终回退：使用 python（会闪一下控制台）
        python "%LAUNCHER%"
    )
)
