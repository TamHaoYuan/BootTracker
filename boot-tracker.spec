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
        # 隧道功能依赖（运行时从 _RESOURCE_DIR 解析）
        ('cloudflared.exe', '.'),
    ],
    datas=[
        ('index.html', '.'),
        ('static', 'static'),
    ],
    hiddenimports=[
        'PyQt5.QtWebEngineWidgets',
        'PyQt5.QtWebEngineCore',
        'pystray._win32',
    ],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[
        # 仅排除与 WebEngine/托盘/组件无关的大模块，减小体积
        'PyQt5.QtBluetooth',
        'PyQt5.QtDesigner',
        'PyQt5.QtLocation',
        'PyQt5.QtMultimedia',
        'PyQt5.QtMultimediaWidgets',
        'PyQt5.QtNfc',
        'PyQt5.QtQuick3D',
        'PyQt5.QtRemoteObjects',
        'PyQt5.QtSensors',
        'PyQt5.QtSerialPort',
        'PyQt5.QtSql',
        'PyQt5.QtTest',
        'PyQt5.QtTextToSpeech',
        'PyQt5.QtXmlPatterns',
        'PyQt5.QtOpenGL',
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
