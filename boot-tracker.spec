# -*- mode: python ; coding: utf-8 -*-
"""PyInstaller 打包配置 — 开机记录桌面应用（onedir 模式）

构建命令： pyinstaller boot-tracker.spec
输出目录： dist/BootTracker/
"""

block_cipher = None

a = Analysis(
    ['boot-tracker.py'],
    pathex=[],
    binaries=[
        # 隧道功能依赖（开发态位于 vendor/，运行时从 _RESOURCE_DIR 解析）
        ('vendor/cloudflared.exe', '.'),
        # 纯原生桌面主程序（egui/eframe，运行时从 desktop/target/release 解析）
        ('desktop/target/release/boot-tracker.exe', 'desktop/target/release'),
        # 纯原生桌面小组件（egui/eframe 浮窗，运行时从 desktop-widget/target/release 解析）
        ('desktop-widget/target/release/boot-tracker-widget.exe', 'desktop-widget/target/release'),
    ],
    datas=[
        # 前端构建产物（Vite build 输出）
        ('dist-static', 'dist-static'),
        # 静态图标与上传目录（托盘图标、背景图）
        ('static', 'static'),
    ],
    hiddenimports=[
        'pystray._win32',
    ],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[
        # 旧版 PyQt5 已弃用（见 deprecated/），整体排除减小体积
        'PyQt5',
        'test',
        'unittest',
        'pydoc',
    ],
    win_no_prefer_redirects=False,
    win_private_assemblies=False,
    cipher=block_cipher,
    noarchive=False,
)
pyz = PYZ(a.pure, a.zipped_data, cipher=block_cipher)

exe = EXE(
    pyz,
    a.scripts,
    [],
    exclude_binaries=True,
    name='开机记录',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=False,
    console=False,  # GUI 应用，无控制台窗口
    icon='icon.ico',
)
coll = COLLECT(
    exe,
    a.binaries,
    a.zipfiles,
    a.datas,
    strip=False,
    upx=False,
    upx_exclude=[],
    name='BootTracker',
)
