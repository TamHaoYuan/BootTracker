# BootTracker 启动脚本（PowerShell 版，对应 开机记录-启动.bat）
# 用法：右键"使用 PowerShell 运行"，或命令行：
#   pwsh -NoProfile -ExecutionPolicy Bypass -File ".\开机记录-启动.ps1"
$ErrorActionPreference = 'Stop'
$scriptDir = $PSScriptRoot

# 启动优先级：源码（开发/最新代码）优先，PyInstaller 打包产物兜底。
# 注意：dist 里的 exe 可能是旧构建（如 Tauri 迁移前的版本），
# 只要项目根目录存在 boot-tracker.py 就始终跑源码版，避免误启旧版。

# 1. 开发模式：Python 启动 boot-tracker.py（Python 再拉起 Tauri 子进程）
$launcher = Join-Path $scriptDir 'boot-tracker.py'
$pythonw = $null
if (Test-Path $launcher) {
    # 解释器查找顺序：项目 venv → PATH 中的 pythonw → 常见安装路径 → python（回退）
    $venvPythonw = Join-Path $scriptDir '.venv\Scripts\pythonw.exe'
    if (Test-Path $venvPythonw) {
        # 优先项目虚拟环境（比 bat 版更智能：无需 pythonw 在 PATH）
        $pythonw = $venvPythonw
    } else {
        $cmd = Get-Command pythonw -ErrorAction SilentlyContinue
        if ($cmd) { $pythonw = $cmd.Source }
    }
    if (-not $pythonw) {
        foreach ($p in @('C:\Python313\pythonw.exe', 'C:\Python312\pythonw.exe')) {
            if (Test-Path $p) { $pythonw = $p; break }
        }
    }
    if ($pythonw) {
        # pythonw 无控制台窗口，启动后本脚本立即退出
        Start-Process -FilePath $pythonw -ArgumentList ('"{0}"' -f $launcher)
        exit 0
    }
    # 最终回退：python（会闪控制台窗口）
    $py = Get-Command python -ErrorAction SilentlyContinue
    if ($py) {
        Start-Process -FilePath $py.Source -ArgumentList ('"{0}"' -f $launcher)
        exit 0
    }
    Write-Host '[ERROR] no python interpreter found (pythonw / python)'
    Read-Host 'Press Enter to exit'
    exit 1
}

# 2. 兜底：打包模式，直接启动 PyInstaller 产物（内含 Python + Tauri 子进程管理）
$packagedExe = Join-Path $scriptDir 'dist\BootTracker\开机记录.exe'
if (Test-Path $packagedExe) {
    Start-Process -FilePath $packagedExe
    exit 0
}

Write-Host "[ERROR] boot-tracker.py not found in: $scriptDir"
Read-Host 'Press Enter to exit'
exit 1
