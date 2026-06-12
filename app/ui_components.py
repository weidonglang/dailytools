from __future__ import annotations

import customtkinter as ctk


class SectionTitle(ctk.CTkLabel):
    def __init__(self, master, text: str, **kwargs) -> None:
        super().__init__(master, text=text, font=ctk.CTkFont(size=18, weight="bold"), anchor="w", **kwargs)


class StatusCard(ctk.CTkFrame):
    def __init__(self, master, title: str) -> None:
        super().__init__(master, corner_radius=10)
        self.grid_columnconfigure(0, weight=1)
        ctk.CTkLabel(self, text=title, font=ctk.CTkFont(size=14, weight="bold"), anchor="w").grid(
            row=0, column=0, padx=16, pady=(12, 2), sticky="ew"
        )
        self.value = ctk.CTkLabel(self, text="未安装", font=ctk.CTkFont(size=19), anchor="w")
        self.value.grid(row=1, column=0, padx=16, pady=2, sticky="ew")
        self.detail = ctk.CTkLabel(self, text="-", text_color=("gray40", "gray65"), anchor="w")
        self.detail.grid(row=2, column=0, padx=16, pady=(2, 12), sticky="ew")

    def update_status(self, value: str, detail: str, ok: bool = False) -> None:
        self.value.configure(text=value, text_color="#16a34a" if ok else ("gray20", "gray85"))
        self.detail.configure(text=detail or "-")


class ScrollablePage(ctk.CTkScrollableFrame):
    def __init__(self, master, **kwargs) -> None:
        super().__init__(master, fg_color="transparent", **kwargs)
        self.grid_columnconfigure(0, weight=1)
