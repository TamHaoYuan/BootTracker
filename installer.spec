# -*- mode: python ; coding: utf-8 -*-
"""安装器打包配置（onefile 单 exe 安装包）

构建命令： pyinstaller installer.spec --noconfirm
输出: dist/BootTracker-Setup.exe
"""
block_cipher = None

a = Analysis(
    ['installer.py'],
    pathex=[],
    binaries=[],
    datas=[
        ('payload.7z', '.'),
        ('7z.exe', '.'),
        ('icon.ico', '.'),
    ],
    hiddenimports=[
        'win32com.client',
        'pythoncom',
        'pywintypes',
    ],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[
        # 安装器仅用 tkinter + win32com，排除应用的大依赖
        'PyQt5',
        'PyQt6',
        'PySide2',
        'PySide6',
        'numpy',
        'PIL',
        'matplotlib',
        'pandas',
        'scipy',
        'tkinter.test',
        'test',
        'unittest',
        'pydoc',
        'distutils',
        'lib2to3',
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
    a.binaries,
    a.datas,
    [],
    name='BootTracker-Setup',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=False,
    runtime_tmpdir=None,
    console=False,  # GUI 安装器
    icon='icon.ico',
)
