from __future__ import annotations

import os
from pathlib import Path
from tkinter import filedialog, messagebox

import customtkinter as ctk

from app.ui_components import ScrollablePage, SectionTitle, StatusCard
from app.ui_env_manager import EnvironmentManagerPage
from app.ui_observability import ObservabilityPage
from app.ui_port_manager import PortManagerPage
from core.config_store import ConfigService
from core.doctor import format_report, run_diagnostics
from core.env_var import configure_user_environment
from core.event_log import EventLog
from core.task_bus import BackgroundTask, TaskBus
from managers.jdk_manager import JdkManager
from managers.node_manager import NodeManager
from managers.python_manager import PythonManager
from observability.runtime_state import collect_runtime_states


ctk.set_default_color_theme("blue")


class Services:
    def __init__(self, app: "DevEnvManagerApp") -> None:
        self.app = app
        self.config = ConfigService()
        ctk.set_appearance_mode(self.config.settings.get("theme", "system"))
        self.tasks = TaskBus()
        self.port_count = 0
        self.log = EventLog(self.config.paths.log_file)
        self._create_managers()

    def _create_managers(self) -> None:
        self.jdk = JdkManager(self.config, self.log)
        self.python = PythonManager(self.config, self.log)
        self.node = NodeManager(self.config, self.log)

    def change_root(self, root: Path) -> None:
        self.config.set_root(root)
        self.log = EventLog(self.config.paths.log_file)
        self._create_managers()
        self.log.write(f"安装根目录已设置为：{root}")

    def submit_task(self, name, func, on_success=None) -> BackgroundTask:
        task = self.tasks.submit(name, func)
        self.log.write(f"开始任务：{name}")
        self.app.show_task(task, on_success)
        return task

    def notify(self, message: str) -> None:
        self.log.write(message)
        self.app.status_label.configure(text=message)

    def refresh_all(self) -> None:
        self.app.refresh_all()


class HomePage(ScrollablePage):
    def __init__(self, master, services: Services) -> None:
        super().__init__(master)
        self.services = services
        self.cards: dict[str, StatusCard] = {}
        self._build()

    def _build(self) -> None:
        SectionTitle(self, text="DevEnv Manager").grid(row=0, column=0, padx=8, pady=(8, 2), sticky="ew")
        ctk.CTkLabel(
            self,
            text="Windows 多版本开发环境管理与端口占用控制工具",
            text_color=("gray40", "gray65"),
            anchor="w",
        ).grid(row=1, column=0, padx=8, pady=(0, 14), sticky="ew")
        cards_frame = ctk.CTkFrame(self, fg_color="transparent")
        cards_frame.grid(row=2, column=0, padx=8, sticky="ew")
        for column, (kind, title) in enumerate((("jdk", "当前 JDK"), ("python", "当前 Python"), ("node", "当前 Node.js"))):
            cards_frame.grid_columnconfigure(column, weight=1)
            card = StatusCard(cards_frame, title)
            card.grid(row=0, column=column, padx=5, sticky="ew")
            self.cards[kind] = card
        info = ctk.CTkFrame(self)
        info.grid(row=3, column=0, padx=8, pady=14, sticky="ew")
        info.grid_columnconfigure(0, weight=1)
        self.root_label = ctk.CTkLabel(info, text="", anchor="w")
        self.root_label.grid(row=0, column=0, padx=15, pady=12, sticky="ew")
        actions = ctk.CTkFrame(self, fg_color="transparent")
        actions.grid(row=4, column=0, padx=8, pady=4, sticky="w")
        ctk.CTkButton(actions, text="一键诊断", command=self.diagnose).pack(side="left", padx=5)
        ctk.CTkButton(actions, text="一键修复 PATH", command=self.fix_path).pack(side="left", padx=5)
        ctk.CTkButton(actions, text="打开安装目录", command=self.open_root).pack(side="left", padx=5)
        ctk.CTkButton(actions, text="打开日志", command=self.open_log).pack(side="left", padx=5)
        SectionTitle(self, text="诊断报告").grid(row=5, column=0, padx=8, pady=(18, 6), sticky="ew")
        self.report = ctk.CTkTextbox(self, height=300)
        self.report.grid(row=6, column=0, padx=8, pady=(0, 8), sticky="ew")
        self.report.insert("1.0", "点击“一键诊断”检查环境。")
        self.report.configure(state="disabled")
        self.refresh()

    def refresh(self) -> None:
        for state in collect_runtime_states(self.services.config):
            self.cards[state.kind].update_status(state.version or "未激活", state.path or "尚未安装", state.healthy)
        self.root_label.configure(text=f"安装根目录：{self.services.config.paths.root}")

    def diagnose(self) -> None:
        def work(update):
            update(20, "正在检查环境变量")
            result = run_diagnostics(self.services.config.paths)
            update(90, "正在生成报告")
            return result
        self.services.submit_task("环境诊断", work, self._show_report)

    def _show_report(self, result) -> None:
        report = format_report(result)
        self.report.configure(state="normal")
        self.report.delete("1.0", "end")
        self.report.insert("1.0", report)
        self.report.configure(state="disabled")
        self.services.log.write("环境诊断完成")

    def fix_path(self) -> None:
        try:
            configure_user_environment(self.services.config.paths)
            self.services.notify("用户环境变量已修复；请重新打开终端或 IDE 后验证")
            messagebox.showinfo("配置完成", "已更新用户级 DEVENV_HOME、JAVA_HOME 和 PATH。\n已打开的终端或 IDE 需要重启。")
        except Exception as exc:
            messagebox.showerror("配置失败", str(exc))

    def open_root(self) -> None:
        self.services.config.paths.ensure()
        os.startfile(self.services.config.paths.root)

    def open_log(self) -> None:
        self.services.config.paths.log_file.touch(exist_ok=True)
        os.startfile(self.services.config.paths.log_file)


class SettingsPage(ScrollablePage):
    def __init__(self, master, services: Services) -> None:
        super().__init__(master)
        self.services = services
        self._build()

    def _build(self) -> None:
        SectionTitle(self, text="设置").grid(row=0, column=0, padx=8, pady=(8, 14), sticky="ew")
        root_frame = ctk.CTkFrame(self)
        root_frame.grid(row=1, column=0, padx=8, pady=6, sticky="ew")
        root_frame.grid_columnconfigure(0, weight=1)
        self.root_entry = ctk.CTkEntry(root_frame)
        self.root_entry.grid(row=0, column=0, padx=10, pady=12, sticky="ew")
        self.root_entry.insert(0, str(self.services.config.paths.root))
        ctk.CTkButton(root_frame, text="浏览", width=80, command=self.browse).grid(row=0, column=1, padx=5)
        ctk.CTkButton(root_frame, text="应用根目录", width=100, command=self.apply_root).grid(row=0, column=2, padx=10)
        theme_frame = ctk.CTkFrame(self)
        theme_frame.grid(row=2, column=0, padx=8, pady=6, sticky="ew")
        ctk.CTkLabel(theme_frame, text="外观主题").pack(side="left", padx=12, pady=12)
        self.theme = ctk.CTkOptionMenu(theme_frame, values=["system", "light", "dark"], command=self.change_theme)
        self.theme.set(self.services.config.settings.get("theme", "system"))
        self.theme.pack(side="left", padx=8)
        ctk.CTkLabel(
            self,
            text="只修改当前用户环境变量，不修改系统级环境变量。运行时均安装在上述根目录内。",
            text_color=("gray40", "gray65"),
            anchor="w",
        ).grid(row=3, column=0, padx=12, pady=12, sticky="ew")

    def browse(self) -> None:
        selected = filedialog.askdirectory(initialdir=self.root_entry.get())
        if selected:
            self.root_entry.delete(0, "end")
            self.root_entry.insert(0, selected)

    def apply_root(self) -> None:
        root = Path(self.root_entry.get().strip())
        if not root.is_absolute():
            messagebox.showerror("路径无效", "请选择绝对路径。")
            return
        try:
            self.services.change_root(root)
            self.services.app.rebuild_pages("settings")
            messagebox.showinfo("设置完成", "安装根目录已更新。")
        except Exception as exc:
            messagebox.showerror("设置失败", str(exc))

    def change_theme(self, value: str) -> None:
        ctk.set_appearance_mode(value)
        self.services.config.save_setting("theme", value)


class DevEnvManagerApp(ctk.CTk):
    def __init__(self) -> None:
        super().__init__()
        self.title("DevEnv Manager")
        self.geometry("1400x850")
        self.minsize(1100, 700)
        self.services = Services(self)
        self.pages: dict[str, ctk.CTkFrame] = {}
        self.nav_buttons: dict[str, ctk.CTkButton] = {}
        self._build_shell()
        self.rebuild_pages(self.services.config.settings.get("last_page", "home"))

    def _build_shell(self) -> None:
        self.grid_columnconfigure(1, weight=1)
        self.grid_rowconfigure(0, weight=1)
        nav = ctk.CTkFrame(self, width=190, corner_radius=0)
        nav.grid(row=0, column=0, sticky="nsew")
        nav.grid_propagate(False)
        ctk.CTkLabel(nav, text="DevEnv\nManager", font=ctk.CTkFont(size=22, weight="bold")).pack(
            padx=18, pady=(28, 30)
        )
        for key, text in (
            ("home", "首页"),
            ("environment", "环境管理"),
            ("observability", "可观测面板"),
            ("ports", "端口管理器"),
            ("settings", "设置"),
        ):
            button = ctk.CTkButton(
                nav, text=text, height=42, anchor="w", fg_color="transparent",
                text_color=("gray10", "gray90"), command=lambda page=key: self.show_page(page)
            )
            button.pack(fill="x", padx=12, pady=4)
            self.nav_buttons[key] = button
        self.content = ctk.CTkFrame(self, corner_radius=0, fg_color=("gray95", "gray10"))
        self.content.grid(row=0, column=1, sticky="nsew")
        self.content.grid_columnconfigure(0, weight=1)
        self.content.grid_rowconfigure(0, weight=1)
        self.status_label = ctk.CTkLabel(self, text="就绪", anchor="w")
        self.status_label.grid(row=1, column=0, columnspan=2, padx=12, pady=4, sticky="ew")

    def rebuild_pages(self, target: str = "home") -> None:
        for page in self.pages.values():
            page.destroy()
        self.pages = {
            "home": HomePage(self.content, self.services),
            "environment": EnvironmentManagerPage(self.content, self.services),
            "observability": ObservabilityPage(self.content, self.services),
            "ports": PortManagerPage(self.content, self.services),
            "settings": SettingsPage(self.content, self.services),
        }
        self.show_page(target if target in self.pages else "home")

    def show_page(self, name: str) -> None:
        for page in self.pages.values():
            page.grid_forget()
        self.pages[name].grid(row=0, column=0, sticky="nsew")
        for key, button in self.nav_buttons.items():
            button.configure(fg_color=("gray75", "gray25") if key == name else "transparent")
        self.services.config.save_setting("last_page", name)
        refresh = getattr(self.pages[name], "refresh", None)
        if refresh:
            refresh()

    def show_task(self, task: BackgroundTask, on_success=None) -> None:
        def poll() -> None:
            self.status_label.configure(text=f"{task.name}：{task.message}  {task.progress}%")
            if task.status in {"pending", "running"}:
                self.after(150, poll)
                return
            if task.status == "success":
                self.services.log.write(f"任务完成：{task.name}")
                if on_success:
                    on_success(task.result) if task.result is not None else on_success()
                self.refresh_all()
            else:
                self.services.log.write(f"任务失败：{task.name} - {task.error}", "error")
                messagebox.showerror("任务失败", task.error or "未知错误")
            self.status_label.configure(text=f"{task.name}：{'完成' if task.status == 'success' else '失败'}")
        self.after(150, poll)

    def refresh_all(self) -> None:
        for page in self.pages.values():
            refresh = getattr(page, "refresh", None)
            if refresh:
                try:
                    refresh()
                except Exception:
                    continue
