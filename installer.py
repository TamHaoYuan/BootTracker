#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""开机记录 — 自包含安装器

功能：
- tkinter 安装界面（无外部依赖，体积小）
- 解压内嵌 payload.7z 到用户选择的目录
- 创建开始菜单/桌面快捷方式、开机自启
- 写入注册表卸载信息 + 生成 uninstall.bat
- 安装后可选立即启动

打包：pyinstaller installer.spec --noconfirm
"""
import os
import sys
import subprocess
import threading
import tkinter as tk
from tkinter import ttk, filedialog, messagebox

APP_NAME = "开机记录"
APP_NAME_EN = "BootTracker"
APP_VERSION = "1.0.0"
EXE_NAME = "开机记录.exe"
PAYLOAD = "payload.7z"
SEVENZ = "7z.exe"


def _res_dir():
    """获取资源目录（onefile 时为 _MEIPASS 临时解压目录）"""
    if getattr(sys, "frozen", False):
        return sys._MEIPASS
    return os.path.dirname(os.path.abspath(__file__))


def _default_dir():
    return os.path.join(
        os.environ.get("LOCALAPPDATA", os.path.expanduser("~")),
        "Programs", APP_NAME_EN,
    )


def _create_shortcut(lnk, target, desc="", icon=""):
    """用 win32com 创建 .lnk 快捷方式"""
    try:
        import win32com.client
        shell = win32com.client.Dispatch("WScript.Shell")
        sc = shell.CreateShortcut(lnk)
        sc.TargetPath = target
        sc.WorkingDirectory = os.path.dirname(target)
        sc.Description = desc
        if icon and os.path.exists(icon):
            sc.IconLocation = icon
        sc.Save()
        return True
    except Exception:
        return False


def _make_uninstall(install_dir, exe):
    """生成卸载脚本并注册到控制面板"""
    bat = os.path.join(install_dir, "uninstall.bat")
    # 卸载脚本：杀进程、删目录、删快捷方式、删注册表
    content = (
        "@echo off\n"
        "chcp 65001 >nul\n"
        f"echo 正在卸载 {APP_NAME}...\n"
        f'taskkill /IM "{EXE_NAME}" /F >nul 2>&1\n'
        "timeout /t 1 /nobreak >nul\n"
        f'rd /s /q "{install_dir}"\n'
        f'del /q "%APPDATA%\\Microsoft\\Windows\\Start Menu\\Programs\\{APP_NAME}.lnk" >nul 2>&1\n'
        f'del /q "%USERPROFILE%\\Desktop\\{APP_NAME}.lnk" >nul 2>&1\n'
        f'del /q "%APPDATA%\\Microsoft\\Windows\\Start Menu\\Programs\\Startup\\{APP_NAME}.lnk" >nul 2>&1\n'
        f'reg delete "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{APP_NAME_EN}" /f >nul 2>&1\n'
        "echo 卸载完成。\n"
        "timeout /t 2 /nobreak >nul\n"
    )
    with open(bat, "w", encoding="utf-8") as f:
        f.write(content)
    # 注册到“应用与功能”面板
    try:
        import winreg
        key = winreg.CreateKey(
            winreg.HKEY_CURRENT_USER,
            f"Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{APP_NAME_EN}",
        )
        winreg.SetValueEx(key, "DisplayName", 0, winreg.REG_SZ, APP_NAME)
        winreg.SetValueEx(key, "DisplayVersion", 0, winreg.REG_SZ, APP_VERSION)
        winreg.SetValueEx(key, "Publisher", 0, winreg.REG_SZ, APP_NAME_EN)
        winreg.SetValueEx(key, "UninstallString", 0, winreg.REG_SZ, bat)
        winreg.SetValueEx(key, "InstallLocation", 0, winreg.REG_SZ, install_dir)
        winreg.SetValueEx(key, "DisplayIcon", 0, winreg.REG_SZ, exe)
        winreg.CloseKey(key)
    except Exception:
        pass


class InstallerApp:
    """安装器主界面"""

    def __init__(self):
        self.root = tk.Tk()
        self.root.title(f"{APP_NAME} 安装程序")
        self.root.geometry("580x460")
        self.root.minsize(520, 420)
        try:
            self.root.iconbitmap(default=os.path.join(_res_dir(), "icon.ico"))
        except Exception:
            pass

        self.install_dir = tk.StringVar(value=_default_dir())
        self.desktop = tk.BooleanVar(value=True)
        self.autostart = tk.BooleanVar(value=True)
        self.installing = False

        self._build_ui()

    def _build_ui(self):
        # 顶部标题
        header = ttk.Frame(self.root, padding=(22, 20, 22, 8))
        header.pack(fill="x")
        ttk.Label(
            header, text=f"安装 {APP_NAME}",
            font=("Microsoft YaHei", 16, "bold"),
        ).pack(anchor="w")
        ttk.Label(
            header, text=f"版本 {APP_VERSION}  ·  自包含安装包",
            font=("Microsoft YaHei", 9),
        ).pack(anchor="w")

        body = ttk.Frame(self.root, padding=(22, 4, 22, 10))
        body.pack(fill="both", expand=True)

        # 安装位置
        ttk.Label(body, text="安装位置：").pack(anchor="w", pady=(8, 2))
        path_row = ttk.Frame(body)
        path_row.pack(fill="x")
        ttk.Entry(path_row, textvariable=self.install_dir).pack(
            side="left", fill="x", expand=True
        )
        ttk.Button(path_row, text="浏览...", command=self._browse).pack(
            side="left", padx=(8, 0)
        )

        # 选项
        opts = ttk.Frame(body)
        opts.pack(fill="x", pady=(14, 0))
        ttk.Checkbutton(
            opts, text="创建桌面快捷方式", variable=self.desktop
        ).pack(anchor="w")
        ttk.Checkbutton(
            opts, text="开机自动启动", variable=self.autostart
        ).pack(anchor="w")

        # 进度
        self.progress = ttk.Progressbar(body, mode="determinate")
        self.progress.pack(fill="x", pady=(18, 6))
        self.status = ttk.Label(
            body, text="准备安装", font=("Microsoft YaHei", 9)
        )
        self.status.pack(anchor="w")

        # 日志
        self.log = tk.Text(
            body, height=8, font=("Consolas", 9),
            state="disabled", wrap="word", relief="sunken", borderwidth=1,
        )
        self.log.pack(fill="both", expand=True, pady=(10, 0))

        # 底部按钮
        foot = ttk.Frame(self.root, padding=(22, 0, 22, 16))
        foot.pack(fill="x")
        self.btn = ttk.Button(foot, text="安装", command=self._start)
        self.btn.pack(side="right")
        ttk.Button(foot, text="取消", command=self._cancel).pack(
            side="right", padx=(0, 8)
        )

    def _browse(self):
        d = filedialog.askdirectory(initialdir=self.install_dir.get())
        if d:
            self.install_dir.set(d)

    def _log(self, msg):
        self.log.configure(state="normal")
        self.log.insert("end", msg + "\n")
        self.log.see("end")
        self.log.configure(state="disabled")
        self.root.update_idletasks()

    def _set_status(self, s, value=None):
        self.status.configure(text=s)
        if value is not None:
            self.progress["value"] = value
        self.root.update_idletasks()

    def _start(self):
        if self.installing:
            return
        target = self.install_dir.get().strip()
        if not target:
            messagebox.showwarning(APP_NAME, "请选择安装位置")
            return
        exe = os.path.join(target, EXE_NAME)
        if os.path.exists(exe):
            if not messagebox.askyesno(
                APP_NAME, "检测到目标目录已存在应用。\n是否覆盖安装？"
            ):
                return
        self.installing = True
        self.btn.configure(state="disabled", text="安装中...")
        threading.Thread(
            target=self._install, args=(target,), daemon=True
        ).start()

    def _install(self, target):
        try:
            os.makedirs(target, exist_ok=True)

            # 1. 解压 payload
            payload = os.path.join(_res_dir(), PAYLOAD)
            sevenz = os.path.join(_res_dir(), SEVENZ)
            if not os.path.exists(payload):
                raise FileNotFoundError("payload.7z 缺失，安装包损坏")
            if not os.path.exists(sevenz):
                raise FileNotFoundError("7z.exe 缺失，安装包损坏")

            self._set_status("正在解压文件...", 10)
            self._log(f"解压到 {target}")
            proc = subprocess.run(
                [sevenz, "x", payload, f"-o{target}", "-y",
                 "-bso0", "-bsp0"],
                capture_output=True,
            )
            if proc.returncode != 0:
                err = proc.stderr.decode("utf-8", "ignore")
                raise RuntimeError(err or "解压失败")

            exe = os.path.join(target, EXE_NAME)
            if not os.path.exists(exe):
                raise FileNotFoundError(f"解压后未找到 {EXE_NAME}")

            # 2. 快捷方式
            self._set_status("创建快捷方式...", 75)
            start_menu = os.path.join(
                os.environ["APPDATA"],
                "Microsoft", "Windows", "Start Menu", "Programs",
            )
            os.makedirs(start_menu, exist_ok=True)
            _create_shortcut(
                os.path.join(start_menu, f"{APP_NAME}.lnk"),
                exe, APP_NAME, exe,
            )
            if self.desktop.get():
                desktop = os.path.join(
                    os.environ["USERPROFILE"], "Desktop"
                )
                _create_shortcut(
                    os.path.join(desktop, f"{APP_NAME}.lnk"),
                    exe, APP_NAME, exe,
                )
            if self.autostart.get():
                startup = os.path.join(
                    os.environ["APPDATA"],
                    "Microsoft", "Windows", "Start Menu", "Programs",
                    "Startup",
                )
                os.makedirs(startup, exist_ok=True)
                _create_shortcut(
                    os.path.join(startup, f"{APP_NAME}.lnk"),
                    exe, APP_NAME, exe,
                )
            self._log("快捷方式已创建")

            # 3. 卸载信息
            self._set_status("注册卸载信息...", 90)
            _make_uninstall(target, exe)
            self._log("卸载程序已注册")

            # 4. 完成
            self._set_status("安装完成！", 100)
            self._log("安装完成。")
            launch = messagebox.askyesno(
                APP_NAME, "安装完成！\n是否立即启动开机记录？"
            )
            if launch:
                subprocess.Popen([exe])
            self.root.after(0, self._done)
        except Exception as e:
            self._log(f"失败: {e}")
            self._set_status("安装失败")
            self.root.after(
                0,
                lambda: (
                    messagebox.showerror(APP_NAME, f"安装失败:\n{e}"),
                    self.btn.configure(state="normal", text="安装"),
                ),
            )
            self.installing = False

    def _done(self):
        self.btn.configure(text="完成", command=self.root.destroy)
        self.btn.configure(state="normal")

    def _cancel(self):
        if self.installing:
            if not messagebox.askyesno(
                APP_NAME, "安装进行中，确定退出？"
            ):
                return
        self.root.destroy()

    def run(self):
        self.root.mainloop()


if __name__ == "__main__":
    InstallerApp().run()
