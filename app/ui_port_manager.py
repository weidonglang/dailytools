from __future__ import annotations

import ctypes
import threading
from tkinter import messagebox, ttk

import customtkinter as ctk

from port.port_models import PortRecord
from port.port_scanner import filter_records, scan_ports
from port.process_control import kill_process


class PortManagerPage(ctk.CTkFrame):
    COLUMNS = ("protocol", "local", "port", "remote", "status", "pid", "name")
    HEADINGS = ("协议", "本地地址", "端口", "远程地址", "状态", "PID", "进程名")

    def __init__(self, master, services) -> None:
        super().__init__(master, fg_color="transparent")
        self.services = services
        self.records: list[PortRecord] = []
        self.filtered: list[PortRecord] = []
        self.scanning = False
        self.auto_job = None
        self.sort_reverse: dict[str, bool] = {}
        self._build()
        self.after(300, self.scan)

    def _build(self) -> None:
        self.grid_columnconfigure(0, weight=1)
        self.grid_rowconfigure(5, weight=1)
        title_row = ctk.CTkFrame(self, fg_color="transparent")
        title_row.grid(row=0, column=0, padx=8, pady=(8, 0), sticky="ew")
        title_row.grid_columnconfigure(0, weight=1)
        ctk.CTkLabel(title_row, text="端口占用搜索控制面板", font=ctk.CTkFont(size=18, weight="bold")).grid(
            row=0, column=0, sticky="w"
        )
        admin = bool(ctypes.windll.shell32.IsUserAnAdmin())
        ctk.CTkLabel(title_row, text=f"当前权限：{'管理员' if admin else '普通用户，部分进程可能无法结束'}").grid(
            row=0, column=1, sticky="e"
        )
        ctk.CTkLabel(
            self,
            text="扫描本机 TCP/UDP 端口，支持搜索、筛选、排序并安全结束指定进程。",
            text_color=("gray40", "gray65"),
            anchor="w",
        ).grid(row=1, column=0, padx=8, pady=(0, 8), sticky="ew")
        actions = ctk.CTkFrame(self)
        actions.grid(row=2, column=0, padx=8, pady=4, sticky="ew")
        actions.grid_columnconfigure(0, weight=1)
        self.search_var = ctk.StringVar()
        search = ctk.CTkEntry(actions, textvariable=self.search_var, placeholder_text="端口 / PID / 进程名 / 协议 / 状态")
        search.grid(row=0, column=0, padx=8, pady=8, sticky="ew")
        search.bind("<KeyRelease>", lambda _event: self.apply_filter())
        ctk.CTkButton(actions, text="扫描 / 刷新  F5", width=120, command=self.scan).grid(row=0, column=1, padx=4)
        ctk.CTkButton(actions, text="结束选中  Del", width=115, fg_color="#b91c1c", command=self.kill_selected).grid(
            row=0, column=2, padx=4
        )
        ctk.CTkButton(actions, text="清空搜索  Esc", width=110, command=self.clear_search).grid(row=0, column=3, padx=4)
        ctk.CTkButton(actions, text="复制选中信息", width=110, command=self.copy_selected).grid(row=0, column=4, padx=8)
        filters = ctk.CTkFrame(self, fg_color="transparent")
        filters.grid(row=3, column=0, padx=8, pady=4, sticky="ew")
        self.listen_var = ctk.BooleanVar(value=False)
        self.hide_var = ctk.BooleanVar(value=True)
        self.auto_var = ctk.BooleanVar(value=False)
        ctk.CTkCheckBox(filters, text="只看监听端口", variable=self.listen_var, command=self.apply_filter).pack(side="left", padx=6)
        ctk.CTkCheckBox(filters, text="隐藏 PID 0/4", variable=self.hide_var, command=self.apply_filter).pack(side="left", padx=6)
        ctk.CTkCheckBox(filters, text="10 秒自动刷新", variable=self.auto_var, command=self._toggle_auto).pack(side="left", padx=6)
        shortcuts = ctk.CTkFrame(self, fg_color="transparent")
        shortcuts.grid(row=4, column=0, padx=8, pady=3, sticky="ew")
        for text, value in (
            ("Spring 8080", "8080"), ("Vite 5173", "5173"), ("React 3000", "3000"),
            ("MySQL 3306", "3306"), ("Redis 6379", "6379"), ("PostgreSQL 5432", "5432"),
            ("MongoDB 27017", "27017"), ("Nginx 80", "80"), ("HTTPS 443", "443"),
            ("java", "java"), ("node", "node"), ("python", "python"), ("nginx", "nginx"),
        ):
            ctk.CTkButton(
                shortcuts, text=text, height=25, width=78, fg_color="transparent", border_width=1,
                text_color=("gray10", "gray90"), command=lambda query=value: self.set_search(query)
            ).pack(side="left", padx=2, pady=2)
        table_frame = ctk.CTkFrame(self)
        table_frame.grid(row=5, column=0, padx=8, pady=5, sticky="nsew")
        table_frame.grid_columnconfigure(0, weight=1)
        table_frame.grid_rowconfigure(0, weight=1)
        self.tree = ttk.Treeview(table_frame, columns=self.COLUMNS, show="headings", selectmode="extended")
        widths = (65, 160, 70, 180, 95, 80, 160)
        for column, heading, width in zip(self.COLUMNS, self.HEADINGS, widths):
            self.tree.heading(column, text=heading, command=lambda c=column: self.sort_by(c))
            self.tree.column(column, width=width, minwidth=55, anchor="center")
        scrollbar = ttk.Scrollbar(table_frame, orient="vertical", command=self.tree.yview)
        self.tree.configure(yscrollcommand=scrollbar.set)
        self.tree.grid(row=0, column=0, sticky="nsew")
        scrollbar.grid(row=0, column=1, sticky="ns")
        self.tree.bind("<<TreeviewSelect>>", self._show_details)
        self.tree.bind("<Double-1>", self._show_details)
        detail_frame = ctk.CTkFrame(self)
        detail_frame.grid(row=6, column=0, padx=8, pady=(3, 8), sticky="ew")
        self.details = ctk.CTkTextbox(detail_frame, height=105)
        self.details.pack(fill="x", padx=8, pady=8)
        self.details.insert("1.0", "未选择进程。")
        self.details.configure(state="disabled")
        self.status = ctk.CTkLabel(self, text="点击“扫描”开始。", anchor="w")
        self.status.grid(row=7, column=0, padx=12, pady=(0, 4), sticky="ew")
        top = self.winfo_toplevel()
        top.bind("<F5>", lambda _event: self.scan())
        top.bind("<Delete>", lambda _event: self.kill_selected())
        top.bind("<Escape>", lambda _event: self.clear_search())

    def scan(self) -> None:
        if self.scanning:
            return
        self.scanning = True
        self.status.configure(text="正在扫描端口...")

        def worker() -> None:
            try:
                records = scan_ports()
                self.after(0, lambda: self._scan_complete(records, None))
            except Exception as exc:
                self.after(0, lambda: self._scan_complete([], str(exc)))

        threading.Thread(target=worker, daemon=True).start()

    def _scan_complete(self, records: list[PortRecord], error: str | None) -> None:
        self.scanning = False
        if error:
            self.status.configure(text=f"扫描失败：{error}")
            self.services.log.write(f"端口扫描失败：{error}", "error")
            return
        self.records = records
        self.services.port_count = len(records)
        self.apply_filter()
        self.status.configure(text=f"扫描完成，共发现 {len(records)} 条记录。安全提示：PID 0/4 和关键系统进程禁止结束。")
        self.services.log.write(f"端口扫描完成，共发现 {len(records)} 条记录")

    def apply_filter(self) -> None:
        self.filtered = filter_records(self.records, self.search_var.get(), self.listen_var.get(), self.hide_var.get())
        selected_keys = {
            (self.tree.set(item, "pid"), self.tree.set(item, "port")) for item in self.tree.selection()
        }
        self.tree.delete(*self.tree.get_children())
        for index, record in enumerate(self.filtered):
            item = self.tree.insert("", "end", iid=str(index), values=record.display_values())
            if (str(record.pid) if record.pid is not None else "-", str(record.local_port)) in selected_keys:
                self.tree.selection_add(item)

    def sort_by(self, column: str) -> None:
        index = self.COLUMNS.index(column)
        reverse = self.sort_reverse.get(column, False)
        def key(record: PortRecord):
            value = record.display_values()[index]
            return int(value) if column in {"port", "pid"} and value.isdigit() else value.casefold()
        self.filtered.sort(key=key, reverse=reverse)
        self.sort_reverse[column] = not reverse
        self.tree.delete(*self.tree.get_children())
        for row, record in enumerate(self.filtered):
            self.tree.insert("", "end", iid=str(row), values=record.display_values())

    def selected_records(self) -> list[PortRecord]:
        result = []
        for item in self.tree.selection():
            try:
                result.append(self.filtered[int(item)])
            except (ValueError, IndexError):
                continue
        return result

    def _show_details(self, _event=None) -> None:
        selected = self.selected_records()
        text = selected[0].details() if selected else "未选择进程。"
        self.details.configure(state="normal")
        self.details.delete("1.0", "end")
        self.details.insert("1.0", text)
        self.details.configure(state="disabled")

    def copy_selected(self) -> None:
        records = self.selected_records()
        if not records:
            messagebox.showinfo("提示", "请先选择一条记录。")
            return
        text = "\n\n".join(record.details() for record in records)
        self.clipboard_clear()
        self.clipboard_append(text)
        self.status.configure(text="选中信息已复制到剪贴板。")

    def kill_selected(self) -> None:
        records = self.selected_records()
        pids = sorted({record.pid for record in records if record.pid is not None})
        if not pids:
            messagebox.showinfo("提示", "选中记录没有可结束的 PID。")
            return
        if not messagebox.askyesno("确认结束进程", f"确定结束 PID {', '.join(map(str, pids))}？这可能导致对应服务停止。"):
            return
        messages = []
        for pid in pids:
            result = kill_process(pid)
            if result.needs_force and messagebox.askyesno("进程未退出", f"{result.message}\n是否强制结束？"):
                result = kill_process(pid, force=True)
            messages.append(result.message)
            self.services.log.write(result.message, "info" if result.success else "warning")
        messagebox.showinfo("处理结果", "\n".join(messages))
        self.scan()

    def set_search(self, value: str) -> None:
        self.search_var.set(value)
        self.apply_filter()

    def clear_search(self) -> None:
        self.search_var.set("")
        self.apply_filter()

    def _toggle_auto(self) -> None:
        if self.auto_job:
            self.after_cancel(self.auto_job)
            self.auto_job = None
        if self.auto_var.get():
            self._auto_tick()

    def _auto_tick(self) -> None:
        if self.auto_var.get():
            self.scan()
            self.auto_job = self.after(10000, self._auto_tick)
