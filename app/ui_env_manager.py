from __future__ import annotations

from pathlib import Path
from tkinter import filedialog, messagebox

import customtkinter as ctk

from app.ui_components import ScrollablePage, SectionTitle


class EnvironmentManagerPage(ScrollablePage):
    def __init__(self, master, services) -> None:
        super().__init__(master)
        self.services = services
        self.status_labels: dict[tuple[str, str], ctk.CTkLabel] = {}
        self._build()

    def _build(self) -> None:
        SectionTitle(self, text="开发环境管理").grid(row=0, column=0, padx=8, pady=(8, 2), sticky="ew")
        ctk.CTkLabel(
            self,
            text="运行时自动从官方源下载到指定根目录。安装完成后自动激活当前版本。",
            anchor="w",
            text_color=("gray40", "gray65"),
        ).grid(row=1, column=0, padx=8, pady=(0, 12), sticky="ew")
        self.root_label = ctk.CTkLabel(self, text=f"安装根目录：{self.services.config.paths.root}", anchor="w")
        self.root_label.grid(row=2, column=0, padx=8, pady=6, sticky="ew")
        rows = (
            ("JDK", "jdk", self.services.jdk, ("17", "21")),
            ("Python", "python", self.services.python, ("3.10", "3.11")),
            ("Node.js", "node", self.services.node, ("20", "22")),
        )
        for index, (title, kind, manager, versions) in enumerate(rows, start=3):
            frame = ctk.CTkFrame(self)
            frame.grid(row=index, column=0, padx=8, pady=8, sticky="ew")
            frame.grid_columnconfigure(0, weight=1)
            ctk.CTkLabel(frame, text=title, font=ctk.CTkFont(size=16, weight="bold")).grid(
                row=0, column=0, padx=14, pady=12, sticky="w"
            )
            for column, version in enumerate(versions, start=1):
                status = ctk.CTkLabel(frame, text="")
                status.grid(row=0, column=column, padx=6)
                self.status_labels[(kind, version)] = status
                ctk.CTkButton(
                    frame,
                    text=f"安装 {version}",
                    width=100,
                    command=lambda m=manager, v=version, n=title: self._install(m, v, n),
                ).grid(row=1, column=column, padx=6, pady=(0, 12))
                ctk.CTkButton(
                    frame,
                    text=f"切换 {version}",
                    width=100,
                    fg_color="transparent",
                    border_width=1,
                    text_color=("gray10", "gray90"),
                    command=lambda m=manager, v=version, n=title: self._switch(m, v, n),
                ).grid(row=2, column=column, padx=6, pady=(0, 12))
        self.refresh()

    def _install(self, manager, version: str, title: str) -> None:
        if not messagebox.askyesno("确认安装", f"从官方源下载并安装 {title} {version}？"):
            return

        def work(update):
            return manager.install(version, update)

        self.services.submit_task(f"安装 {title} {version}", work, lambda _result: self.refresh())

    def _switch(self, manager, version: str, title: str) -> None:
        try:
            manager.switch(version)
            self.services.notify(f"已切换 {title} {version}")
            self.services.refresh_all()
        except Exception as exc:
            messagebox.showerror("切换失败", str(exc))

    def refresh(self) -> None:
        self.root_label.configure(text=f"安装根目录：{self.services.config.paths.root}")
        data = self.services.config.installed()
        mapping = {"jdk": "jdks", "python": "pythons", "node": "nodes"}
        for (kind, version), label in self.status_labels.items():
            installed = any(item["version"] == version for item in data[mapping[kind]])
            current = data["current"].get(kind) == version
            label.configure(
                text="当前" if current else ("已安装" if installed else "未安装"),
                text_color="#16a34a" if current else ("#2563eb" if installed else ("gray45", "gray60")),
            )
