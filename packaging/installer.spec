# -*- mode: python ; coding: utf-8 -*-
"""安装器打包配置（onefile 单 exe 安装包）

构建命令（项目根目录执行）： pyinstaller packaging\installer.spec --noconfirm
输出: dist/BootTracker-Setup.exe
"""
import os

block_cipher = None

# spec 位于 packaging/，项目根为其上一级（SPECPATH = 本 spec 所在目录）
_ROOT = os.path.normpath(os.path.join(SPECPATH, '..'))

a = Analysis(
    [os.path.join(SPECPATH, 'installer.py')],
    pathex=[],
    binaries=[],
    datas=[
        (os.path.join(_ROOT, 'payload.7z'), '.'),
        (os.path.join(_ROOT, 'vendor', '7z.exe'), '.'),
        (os.path.join(_ROOT, 'icon.ico'), '.'),
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
    icon=os.path.join(_ROOT, 'icon.ico'),
)
