from __future__ import annotations

import os
from tkinter import messagebox, ttk

import customtkinter as ctk

from app.ui_components import ScrollablePage, SectionTitle
from core.runtime_discovery import RuntimeInstallation, discover_all


TOOL_LABELS = {"jdk": "JDK", "python": "Python", "node": "Node.js"}


class EnvironmentManagerPage(ScrollablePage):
    def __init__(self, master, services) -> None:
        super().__init__(master)
        self.services = services
        self.installations: list[RuntimeInstallation] = []
        self.controls: dict[str, ctk.CTkOptionMenu] = {}
        self._build()
        self.after(400, self.scan_installations)

    def _build(self) -> None:
        SectionTitle(self, text="开发环境管理").grid(row=0, column=0, padx=8, pady=(8, 2), sticky="ew")
        ctk.CTkLabel(
            self,
            text="可安装多个版本并快速切换。系统原有运行时只检测和展示，不会被 DevEnv 删除。",
            anchor="w",
            text_color=("gray40", "gray65"),
        ).grid(row=1, column=0, padx=8, pady=(0, 10), sticky="ew")
        self.root_label = ctk.CTkLabel(self, text="", anchor="w")
        self.root_label.grid(row=2, column=0, padx=8, pady=5, sticky="ew")

        install_frame = ctk.CTkFrame(self)
        install_frame.grid(row=3, column=0, padx=8, pady=8, sticky="ew")
        install_frame.grid_columnconfigure(0, weight=1)
        rows = (
            ("JDK", "jdk", self.services.jdk),
            ("Python", "python", self.services.python),
            ("Node.js", "node", self.services.node),
        )
        for row, (title, kind, manager) in enumerate(rows):
            ctk.CTkLabel(install_frame, text=title, font=ctk.CTkFont(size=15, weight="bold"), width=90).grid(
                row=row, column=0, padx=12, pady=10, sticky="w"
            )
            selector = ctk.CTkOptionMenu(install_frame, values=list(manager.supported_versions), width=130)
            selector.set(manager.supported_versions[-2] if len(manager.supported_versions) > 1 else manager.supported_versions[0])
            selector.grid(row=row, column=1, padx=8)
            self.controls[kind] = selector
            ctk.CTkButton(
                install_frame,
                text="下载安装",
                width=110,
                command=lambda m=manager, k=kind: self._install(m, k),
            ).grid(row=row, column=2, padx=8)
            ctk.CTkLabel(
                install_frame,
                text=self._version_hint(kind),
                anchor="w",
                text_color=("gray40", "gray65"),
            ).grid(row=row, column=3, padx=10, sticky="w")

        header = ctk.CTkFrame(self, fg_color="transparent")
        header.grid(row=4, column=0, padx=8, pady=(12, 4), sticky="ew")
        header.grid_columnconfigure(0, weight=1)
        SectionTitle(header, text="已发现运行时").grid(row=0, column=0, sticky="w")
        ctk.CTkButton(header, text="重新检测", width=90, command=self.scan_installations).grid(row=0, column=1, padx=4)
        ctk.CTkButton(header, text="切换选中", width=90, command=self.switch_selected).grid(row=0, column=2, padx=4)
        ctk.CTkButton(header, text="打开位置", width=90, command=self.open_selected).grid(row=0, column=3, padx=4)
        ctk.CTkButton(
            header, text="卸载受管版本", width=110, fg_color="#b91c1c", command=self.uninstall_selected
        ).grid(row=0, column=4, padx=4)

        table = ctk.CTkFrame(self)
        table.grid(row=5, column=0, padx=8, pady=(0, 8), sticky="nsew")
        table.grid_columnconfigure(0, weight=1)
        table.grid_rowconfigure(0, weight=1)
        columns = ("tool", "version", "source", "status", "path")
        self.tree = ttk.Treeview(table, columns=columns, show="headings", height=13, selectmode="browse")
        headings = ("工具", "检测版本", "来源", "状态", "安装位置")
        widths = (90, 130, 130, 100, 650)
        for column, heading, width in zip(columns, headings, widths):
            self.tree.heading(column, text=heading)
            self.tree.column(column, width=width, minwidth=70, anchor="w")
        scrollbar = ttk.Scrollbar(table, orient="vertical", command=self.tree.yview)
        self.tree.configure(yscrollcommand=scrollbar.set)
        self.tree.grid(row=0, column=0, sticky="nsew")
        scrollbar.grid(row=0, column=1, sticky="ns")
        self.detail = ctk.CTkLabel(
            self,
            text="系统运行时仅供识别；要使用 DevEnv 切换和卸载，请通过上方下载安装。",
            anchor="w",
            text_color=("gray40", "gray65"),
        )
        self.detail.grid(row=6, column=0, padx=10, pady=(0, 8), sticky="ew")
        self.refresh()

    def _version_hint(self, kind: str) -> str:
        return {
            "jdk": "含教学常用 JDK 8、LTS 11/17/21/25",
            "python": "旧版本可能自动选择最后一个 Windows 安装器",
            "node": "含旧项目常用 16/18 和当前 LTS 系列",
        }[kind]

    def _install(self, manager, kind: str) -> None:
        version = self.controls[kind].get()
        title = TOOL_LABELS[kind]
        if any(item.managed and item.kind == kind and item.managed_id == version for item in self.installations):
            messagebox.showinfo("已经安装", f"DevEnv 已管理 {title} {version}，无需重复安装。")
            return
        if not messagebox.askyesno("确认安装", f"从官方源下载并安装 {title} {version}？"):
            return
        self.services.submit_task(
            f"安装 {title} {version}",
            lambda update: manager.install(version, update),
            lambda _result: self.scan_installations(),
        )

    def scan_installations(self) -> None:
        self.services.submit_task("检测已安装运行时", lambda update: self._discover(update), self._show_installations)

    def _discover(self, update):
        update(20, "正在读取注册表和系统 PATH")
        result = discover_all(self.services.config)
        update(90, "正在整理版本和安装位置")
        return result

    def _show_installations(self, installations: list[RuntimeInstallation]) -> None:
        self.installations = installations
        self.tree.delete(*self.tree.get_children())
        for index, item in enumerate(installations):
            status = "当前版本" if item.current else ("DevEnv 管理" if item.managed else "系统已有")
            self.tree.insert(
                "",
                "end",
                iid=str(index),
                values=(TOOL_LABELS[item.kind], item.version, item.source, status, str(item.path)),
            )
        counts = {kind: sum(item.kind == kind for item in installations) for kind in TOOL_LABELS}
        self.detail.configure(
            text=f"检测完成：JDK {counts['jdk']} 个，Python {counts['python']} 个，Node.js {counts['node']} 个。"
        )
        self.services.refresh_all()

    def selected(self) -> RuntimeInstallation | None:
        selection = self.tree.selection()
        if not selection:
            return None
        try:
            return self.installations[int(selection[0])]
        except (ValueError, IndexError):
            return None

    def switch_selected(self) -> None:
        item = self.selected()
        if not item:
            messagebox.showinfo("提示", "请先选择一个运行时。")
            return
        if not item.managed or not item.managed_id:
            messagebox.showinfo("不能切换", "系统已有版本不由 DevEnv 管理。请先通过上方下载安装对应版本。")
            return
        manager = getattr(self.services, item.kind)
        try:
            manager.switch(item.managed_id)
            self.services.notify(f"已切换 {TOOL_LABELS[item.kind]} {item.managed_id}")
            self.scan_installations()
        except Exception as exc:
            messagebox.showerror("切换失败", str(exc))

    def open_selected(self) -> None:
        item = self.selected()
        if not item:
            messagebox.showinfo("提示", "请先选择一个运行时。")
            return
        if not item.path.exists():
            messagebox.showerror("路径不存在", str(item.path))
            return
        os.startfile(item.path)

    def uninstall_selected(self) -> None:
        item = self.selected()
        if not item:
            messagebox.showinfo("提示", "请先选择一个运行时。")
            return
        if not item.managed or not item.managed_id:
            messagebox.showwarning("拒绝卸载", "只能卸载 DevEnv 自己管理的版本，系统已有版本不会被删除。")
            return
        title = TOOL_LABELS[item.kind]
        if not messagebox.askyesno(
            "确认卸载",
            f"确定卸载 {title} {item.managed_id}？\n\n将删除：{item.path}\n系统中其他位置的版本不受影响。",
        ):
            return
        manager = getattr(self.services, item.kind)
        self.services.submit_task(
            f"卸载 {title} {item.managed_id}",
            lambda update: self._uninstall(manager, item.managed_id, update),
            lambda _result: self.scan_installations(),
        )

    @staticmethod
    def _uninstall(manager, version: str, update):
        update(20, "正在检查受管目录")
        result = manager.uninstall(version)
        update(90, "正在更新安装记录")
        return result

    def refresh(self) -> None:
        self.root_label.configure(text=f"安装根目录：{self.services.config.paths.root}")
