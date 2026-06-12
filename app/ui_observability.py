from __future__ import annotations

import customtkinter as ctk

from app.ui_components import ScrollablePage, SectionTitle, StatusCard
from core.doctor import run_diagnostics
from observability.metrics import collect_metrics
from observability.runtime_state import collect_runtime_states


class ObservabilityPage(ScrollablePage):
    def __init__(self, master, services) -> None:
        super().__init__(master)
        self.services = services
        self.cards: dict[str, StatusCard] = {}
        self.metric_labels: dict[str, ctk.CTkLabel] = {}
        self._build()

    def _build(self) -> None:
        SectionTitle(self, text="可观测控制面板").grid(row=0, column=0, padx=8, pady=(8, 10), sticky="ew")
        cards = ctk.CTkFrame(self, fg_color="transparent")
        cards.grid(row=1, column=0, padx=8, sticky="ew")
        for column, (kind, title) in enumerate((("jdk", "JDK"), ("python", "Python"), ("node", "Node.js"))):
            cards.grid_columnconfigure(column, weight=1)
            card = StatusCard(cards, title)
            card.grid(row=0, column=column, padx=5, sticky="ew")
            self.cards[kind] = card
        metrics = ctk.CTkFrame(self)
        metrics.grid(row=2, column=0, padx=8, pady=12, sticky="ew")
        for column in range(6):
            metrics.grid_columnconfigure(column, weight=1)
        for column, name in enumerate(("JDK 数量", "Python 数量", "Node.js 数量", "任务成功", "任务失败", "端口记录")):
            ctk.CTkLabel(metrics, text=name, text_color=("gray40", "gray65")).grid(row=0, column=column, padx=8, pady=(12, 2))
            label = ctk.CTkLabel(metrics, text="0", font=ctk.CTkFont(size=22, weight="bold"))
            label.grid(row=1, column=column, padx=8, pady=(2, 12))
            self.metric_labels[name] = label
        SectionTitle(self, text="最近事件").grid(row=3, column=0, padx=8, pady=(4, 6), sticky="ew")
        self.log_box = ctk.CTkTextbox(self, height=330)
        self.log_box.grid(row=4, column=0, padx=8, pady=(0, 8), sticky="nsew")
        self.refresh()

    def refresh(self) -> None:
        for state in collect_runtime_states(self.services.config):
            self.cards[state.kind].update_status(
                state.version or "未激活",
                state.path or "尚未安装",
                state.healthy,
            )
        metrics = collect_metrics(self.services.config, self.services.tasks, self.services.port_count)
        for name, value in metrics.items():
            self.metric_labels[name].configure(text=str(value))
        self.log_box.configure(state="normal")
        self.log_box.delete("1.0", "end")
        self.log_box.insert("end", "\n".join(self.services.log.items))
        self.log_box.see("end")
        self.log_box.configure(state="disabled")
