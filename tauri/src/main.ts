import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Activity,
  Boxes,
  Download,
  FolderSearch,
  Gauge,
  Hammer,
  Network,
  Play,
  RefreshCw,
  Route,
  Shield,
  Terminal,
  Trash2,
  type IconNode,
} from "lucide";
import "./styles.css";

type AppSnapshot = {
  defaultRoot: string;
  configDir: string;
  os: string;
  arch: string;
  username: string;
};

type EnvSnapshot = {
  pathEntries: string[];
  javaHome?: string;
  devenvHome?: string;
  pathWarnings: string[];
};

type ConfigView = {
  settings: {
    rootDir: string;
    downloadTimeoutSeconds: number;
    theme: string;
  };
  installed: {
    jdks: ManagedRuntime[];
    pythons: ManagedRuntime[];
    nodes: ManagedRuntime[];
    mavens: ManagedRuntime[];
    gradles: ManagedRuntime[];
    current: Record<string, string | null>;
  };
  paths: {
    root: string;
    downloads: string;
    config: string;
    current: string;
  };
};

type ManagedRuntime = {
  version: string;
  path: string;
  detail?: string;
  installed_at?: string;
  installedAt?: string;
};

type OperationResult = {
  success: boolean;
  message: string;
};

type KillResult = OperationResult & {
  needsForce: boolean;
  blocked: boolean;
};

type RuntimeInfo = {
  kind: string;
  version: string;
  executable: string;
  source: string;
};

type PortRecord = {
  protocol: string;
  localAddress: string;
  localPort: number;
  remoteAddress: string;
  state: string;
  pid: number;
  processName: string;
  risk: string;
};

type ProjectHealth = {
  root: string;
  projectTypes: string[];
  signals: string[];
  suggestions: string[];
};

type TaskProgress = {
  task: string;
  percent: number;
  message: string;
};

type NetworkDiagnostics = {
  checks: Array<{
    name: string;
    url: string;
    success: boolean;
    status: string;
    elapsedMs: number;
  }>;
  proxy: Array<[string, string]>;
};

type CacheEntry = {
  name: string;
  path: string;
  size: number;
  sha256?: string;
};

type CommandRunResult = {
  success: boolean;
  returnCode: number;
  output: string;
  elapsedMs: number;
};

const app = document.querySelector<HTMLDivElement>("#app");

if (!app) {
  throw new Error("Missing app root");
}

app.innerHTML = `
  <main class="shell">
    <aside class="sidebar">
      <div class="brand">
        <div class="brand-mark">${icon(Boxes)}</div>
        <div>
          <strong>DevEnv Manager</strong>
          <span>Tauri Preview</span>
        </div>
      </div>
      <nav class="nav">
        <button class="nav-item active" data-view="overview">${icon(Gauge)}<span>总览</span></button>
        <button class="nav-item" data-view="ports">${icon(Network)}<span>端口</span></button>
        <button class="nav-item" data-view="runtimes">${icon(Terminal)}<span>运行时</span></button>
        <button class="nav-item" data-view="environment">${icon(Route)}<span>环境</span></button>
        <button class="nav-item" data-view="project">${icon(FolderSearch)}<span>项目</span></button>
        <button class="nav-item" data-view="toolbox">${icon(Hammer)}<span>工具箱</span></button>
      </nav>
    </aside>
    <section class="workspace">
      <header class="topbar">
        <div>
          <h1>DevEnv Manager</h1>
          <p id="subtitle">轻量 Tauri/Rust 重构预览版</p>
        </div>
        <button id="refresh-all" class="primary">${icon(RefreshCw)}<span>刷新</span></button>
      </header>
      <div id="toast" class="toast" hidden></div>

      <section id="view-overview" class="view active">
        <div class="metrics">
          <article class="metric">
            <span>默认根目录</span>
            <strong id="metric-root">...</strong>
          </article>
          <article class="metric">
            <span>运行时</span>
            <strong id="metric-runtimes">0</strong>
          </article>
          <article class="metric">
            <span>端口记录</span>
            <strong id="metric-ports">0</strong>
          </article>
          <article class="metric">
            <span>PATH 警告</span>
            <strong id="metric-warnings">0</strong>
          </article>
        </div>
        <div class="grid two">
          <section class="panel">
            <div class="panel-title">${icon(Activity)}<h2>系统快照</h2></div>
            <div id="snapshot-list" class="kv-list"></div>
          </section>
          <section class="panel">
            <div class="panel-title">${icon(Shield)}<h2>迁移状态</h2></div>
            <ul class="status-list">
              <li><span class="dot done"></span> Tauri 2 桌面外壳</li>
              <li><span class="dot done"></span> Rust 命令桥接</li>
              <li><span class="dot done"></span> 端口扫描 MVP</li>
              <li><span class="dot done"></span> JDK 下载、安装和切换</li>
              <li><span class="dot done"></span> PATH 修复与恢复</li>
              <li><span class="dot todo"></span> Python / Node / Maven / Gradle 安装链路</li>
            </ul>
          </section>
        </div>
        <section class="panel root-panel">
          <div class="panel-title">${icon(Route)}<h2>安装根目录</h2></div>
          <div class="form-row wide">
            <input id="root-dir" />
            <button id="save-root"><span>保存</span></button>
          </div>
          <div id="root-detail" class="small-note"></div>
        </section>
      </section>

      <section id="view-ports" class="view">
        <section class="panel">
          <div class="panel-head">
            <div class="panel-title">${icon(Network)}<h2>端口管理</h2></div>
            <button id="scan-ports">${icon(RefreshCw)}<span>扫描</span></button>
          </div>
          <div class="table-wrap">
            <table>
              <thead>
                <tr><th>协议</th><th>本地地址</th><th>端口</th><th>状态</th><th>PID</th><th>进程</th><th>风险</th><th>操作</th></tr>
              </thead>
              <tbody id="ports-body"></tbody>
            </table>
          </div>
        </section>
      </section>

      <section id="view-runtimes" class="view">
        <section class="panel">
          <div class="panel-head">
            <div class="panel-title">${icon(Terminal)}<h2>运行时发现</h2></div>
            <button id="discover-runtimes">${icon(RefreshCw)}<span>发现</span></button>
          </div>
          <div id="runtime-list" class="runtime-list"></div>
        </section>
        <section class="panel runtime-manager">
          <div class="panel-title">${icon(Download)}<h2>JDK 管理</h2></div>
          <div class="toolbar">
            <select id="jdk-version">
              <option value="8">JDK 8</option>
              <option value="11">JDK 11</option>
              <option value="17">JDK 17</option>
              <option value="21" selected>JDK 21</option>
              <option value="25">JDK 25</option>
            </select>
            <button id="install-jdk">${icon(Download)}<span>安装</span></button>
          </div>
          <div id="managed-jdks" class="runtime-list"></div>
        </section>
        <section class="panel runtime-manager">
          <div class="panel-title">${icon(Download)}<h2>Node.js 管理</h2></div>
          <div class="toolbar">
            <select id="node-version">
              <option value="16">Node.js 16</option>
              <option value="18">Node.js 18</option>
              <option value="20">Node.js 20</option>
              <option value="22" selected>Node.js 22</option>
              <option value="24">Node.js 24</option>
            </select>
            <button id="install-node">${icon(Download)}<span>安装</span></button>
          </div>
          <div id="managed-nodes" class="runtime-list"></div>
        </section>
        <section class="panel runtime-manager">
          <div class="panel-title">${icon(Download)}<h2>Python 管理</h2></div>
          <div class="toolbar">
            <select id="python-version">
              <option value="3.9">Python 3.9</option>
              <option value="3.10">Python 3.10</option>
              <option value="3.11" selected>Python 3.11</option>
              <option value="3.12">Python 3.12</option>
              <option value="3.13">Python 3.13</option>
              <option value="3.14">Python 3.14</option>
            </select>
            <button id="install-python">${icon(Download)}<span>安装</span></button>
          </div>
          <div id="managed-pythons" class="runtime-list"></div>
        </section>
        <section class="panel runtime-manager">
          <div class="panel-title">${icon(Download)}<h2>构建工具</h2></div>
          <div class="toolbar">
            <button id="install-maven">${icon(Download)}<span>安装 Maven 最新版</span></button>
            <button id="install-gradle">${icon(Download)}<span>安装 Gradle 最新版</span></button>
          </div>
          <div id="managed-build-tools" class="runtime-list"></div>
        </section>
      </section>

      <section id="view-environment" class="view">
        <div class="grid two">
          <section class="panel">
            <div class="panel-title">${icon(Route)}<h2>环境变量</h2></div>
            <div class="toolbar">
              <button id="configure-env">${icon(Shield)}<span>配置</span></button>
              <button id="restore-env">${icon(RefreshCw)}<span>恢复</span></button>
            </div>
            <div id="env-list" class="kv-list"></div>
          </section>
          <section class="panel">
            <div class="panel-title">${icon(Shield)}<h2>PATH 检查</h2></div>
            <div id="path-warnings" class="warning-list"></div>
          </section>
        </div>
      </section>

      <section id="view-toolbox" class="view">
        <div class="grid two">
          <section class="panel">
            <div class="panel-head">
              <div class="panel-title">${icon(Network)}<h2>网络诊断</h2></div>
              <button id="run-network">${icon(RefreshCw)}<span>检查</span></button>
            </div>
            <div id="network-result" class="runtime-list"></div>
          </section>
          <section class="panel">
            <div class="panel-head">
              <div class="panel-title">${icon(Download)}<h2>下载缓存</h2></div>
              <div class="toolbar compact">
                <button id="load-cache">${icon(RefreshCw)}<span>刷新</span></button>
                <button id="clear-cache">${icon(Trash2)}<span>清理</span></button>
              </div>
            </div>
            <div id="cache-list" class="runtime-list"></div>
          </section>
        </div>
        <section class="panel runtime-manager">
          <div class="panel-title">${icon(Terminal)}<h2>命令面板</h2></div>
          <div class="form-row command-row">
            <input id="command-input" value="node --version" />
            <input id="command-cwd" placeholder="工作目录，可留空" />
            <button id="run-command">${icon(Play)}<span>运行</span></button>
          </div>
          <pre id="command-output" class="command-output"></pre>
        </section>
      </section>

      <section id="view-project" class="view">
        <section class="panel">
          <div class="panel-title">${icon(FolderSearch)}<h2>项目健康</h2></div>
          <div class="form-row">
            <input id="project-path" value="E:\\\\pycode\\\\dailytools" />
            <button id="check-project">${icon(Play)}<span>检查</span></button>
          </div>
          <div class="toolbar project-actions">
            <button id="generate-vscode">${icon(Hammer)}<span>生成 VS Code 配置</span></button>
          </div>
          <div id="project-health" class="project-health"></div>
        </section>
      </section>
    </section>
  </main>
`;

const state = {
  snapshot: null as AppSnapshot | null,
  env: null as EnvSnapshot | null,
  config: null as ConfigView | null,
  runtimes: [] as RuntimeInfo[],
  ports: [] as PortRecord[],
  network: null as NetworkDiagnostics | null,
  cache: [] as CacheEntry[],
};

function icon(node: IconNode) {
  const [, attrs, children = []] = node;
  const attrsText = Object.entries({
    xmlns: "http://www.w3.org/2000/svg",
    width: "18",
    height: "18",
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    "stroke-width": "2",
    "stroke-linecap": "round",
    "stroke-linejoin": "round",
    ...attrs,
  })
    .map(([key, value]) => `${key}="${escapeHtml(String(value))}"`)
    .join(" ");
  const childText = children
    .map(([tag, childAttrs]) => {
      const childAttrsText = Object.entries(childAttrs)
        .map(([key, value]) => `${key}="${escapeHtml(String(value))}"`)
        .join(" ");
      return `<${tag} ${childAttrsText}></${tag}>`;
    })
    .join("");
  return `<svg ${attrsText}>${childText}</svg>`;
}

function setText(id: string, value: string | number) {
  const element = document.querySelector<HTMLElement>(`#${id}`);
  if (element) element.textContent = String(value);
}

function renderKeyValues(id: string, values: Array<[string, string | undefined]>) {
  const element = document.querySelector<HTMLElement>(`#${id}`);
  if (!element) return;
  element.innerHTML = values
    .map(([label, value]) => `<div><span>${label}</span><strong>${escapeHtml(value || "未设置")}</strong></div>`)
    .join("");
}

function renderSnapshot() {
  if (!state.snapshot) return;
  setText("metric-root", state.config?.settings.rootDir || state.snapshot.defaultRoot);
  renderKeyValues("snapshot-list", [
    ["用户", state.snapshot.username],
    ["系统", `${state.snapshot.os} / ${state.snapshot.arch}`],
    ["配置目录", state.snapshot.configDir],
    ["默认根目录", state.snapshot.defaultRoot],
  ]);
  const rootInput = document.querySelector<HTMLInputElement>("#root-dir");
  if (rootInput && state.config) rootInput.value = state.config.settings.rootDir;
  const rootDetail = document.querySelector<HTMLElement>("#root-detail");
  if (rootDetail && state.config) {
    rootDetail.textContent = `下载缓存：${state.config.paths.downloads} · 配置：${state.config.paths.config}`;
  }
}

function renderEnv() {
  if (!state.env) return;
  setText("metric-warnings", state.env.pathWarnings.length);
  renderKeyValues("env-list", [
    ["DEVENV_HOME", state.env.devenvHome],
    ["JAVA_HOME", state.env.javaHome],
    ["PATH 条目", `${state.env.pathEntries.length} 个`],
  ]);

  const warnings = document.querySelector<HTMLElement>("#path-warnings");
  if (!warnings) return;
  warnings.innerHTML = state.env.pathWarnings.length
    ? state.env.pathWarnings.map((item) => `<div class="warning">${escapeHtml(item)}</div>`).join("")
    : `<div class="empty">当前进程 PATH 没有发现重复或失效条目</div>`;
}

function renderRuntimes() {
  setText("metric-runtimes", state.runtimes.length);
  const element = document.querySelector<HTMLElement>("#runtime-list");
  if (!element) return;
  element.innerHTML = state.runtimes.length
    ? state.runtimes
        .map(
          (runtime) => `
            <article class="runtime">
              <div><strong>${escapeHtml(runtime.kind)}</strong><span>${escapeHtml(runtime.version)}</span></div>
              <small>${escapeHtml(runtime.source)} · ${escapeHtml(runtime.executable)}</small>
            </article>
          `,
        )
        .join("")
    : `<div class="empty">还没有发现运行时</div>`;
  renderManagedJdks();
  renderManagedNodes();
  renderManagedPythons();
  renderManagedBuildTools();
}

function renderManagedJdks() {
  const element = document.querySelector<HTMLElement>("#managed-jdks");
  if (!element) return;
  const jdks = state.config?.installed.jdks || [];
  const current = state.config?.installed.current.jdk;
  element.innerHTML = jdks.length
    ? jdks
        .map(
          (jdk) => `
            <article class="runtime managed-runtime">
              <div>
                <strong>JDK ${escapeHtml(jdk.version)}${current === jdk.version ? " · 当前" : ""}</strong>
                <span>${escapeHtml(jdk.detail || "")}</span>
              </div>
              <small>${escapeHtml(jdk.path)}</small>
              <div class="row-actions">
                <button data-action="switch-jdk" data-version="${escapeHtml(jdk.version)}">${icon(RefreshCw)}<span>切换</span></button>
                <button data-action="uninstall-jdk" data-version="${escapeHtml(jdk.version)}">${icon(Trash2)}<span>卸载</span></button>
              </div>
            </article>
          `,
        )
        .join("")
    : `<div class="empty">还没有安装受管 JDK</div>`;
}

function renderManagedNodes() {
  const element = document.querySelector<HTMLElement>("#managed-nodes");
  if (!element) return;
  const nodes = state.config?.installed.nodes || [];
  const current = state.config?.installed.current.node;
  element.innerHTML = nodes.length
    ? nodes
        .map(
          (node) => `
            <article class="runtime managed-runtime">
              <div>
                <strong>Node.js ${escapeHtml(node.version)}${current === node.version ? " · 当前" : ""}</strong>
                <span>${escapeHtml(node.detail || "")}</span>
              </div>
              <small>${escapeHtml(node.path)}</small>
              <div class="row-actions">
                <button data-action="switch-node" data-version="${escapeHtml(node.version)}">${icon(RefreshCw)}<span>切换</span></button>
                <button data-action="uninstall-node" data-version="${escapeHtml(node.version)}">${icon(Trash2)}<span>卸载</span></button>
              </div>
            </article>
          `,
        )
        .join("")
    : `<div class="empty">还没有安装受管 Node.js</div>`;
}

function renderManagedPythons() {
  const element = document.querySelector<HTMLElement>("#managed-pythons");
  if (!element) return;
  const pythons = state.config?.installed.pythons || [];
  const current = state.config?.installed.current.python;
  element.innerHTML = pythons.length
    ? pythons
        .map(
          (python) => `
            <article class="runtime managed-runtime">
              <div>
                <strong>Python ${escapeHtml(python.version)}${current === python.version ? " · 当前" : ""}</strong>
                <span>${escapeHtml(python.detail || "")}</span>
              </div>
              <small>${escapeHtml(python.path)}</small>
              <div class="row-actions">
                <button data-action="switch-python" data-version="${escapeHtml(python.version)}">${icon(RefreshCw)}<span>切换</span></button>
                <button data-action="uninstall-python" data-version="${escapeHtml(python.version)}">${icon(Trash2)}<span>卸载</span></button>
              </div>
            </article>
          `,
        )
        .join("")
    : `<div class="empty">还没有安装受管 Python</div>`;
}

function renderManagedBuildTools() {
  const element = document.querySelector<HTMLElement>("#managed-build-tools");
  if (!element) return;
  const items = [
    ...(state.config?.installed.mavens || []).map((item) => ({ ...item, kind: "maven", label: "Maven" })),
    ...(state.config?.installed.gradles || []).map((item) => ({ ...item, kind: "gradle", label: "Gradle" })),
  ];
  const current = state.config?.installed.current || {};
  element.innerHTML = items.length
    ? items
        .map(
          (item) => `
            <article class="runtime managed-runtime">
              <div>
                <strong>${item.label} ${escapeHtml(item.version)}${current[item.kind] === item.version ? " · 当前" : ""}</strong>
                <span>${escapeHtml(item.detail || "")}</span>
              </div>
              <small>${escapeHtml(item.path)}</small>
              <div class="row-actions">
                <button data-action="switch-build-tool" data-kind="${item.kind}" data-version="${escapeHtml(item.version)}">${icon(RefreshCw)}<span>切换</span></button>
                <button data-action="uninstall-build-tool" data-kind="${item.kind}" data-version="${escapeHtml(item.version)}">${icon(Trash2)}<span>卸载</span></button>
              </div>
            </article>
          `,
        )
        .join("")
    : `<div class="empty">还没有安装受管 Maven 或 Gradle</div>`;
}

function renderPorts() {
  setText("metric-ports", state.ports.length);
  const body = document.querySelector<HTMLElement>("#ports-body");
  if (!body) return;
  body.innerHTML = state.ports
    .slice(0, 250)
    .map(
      (record) => `
        <tr>
          <td>${escapeHtml(record.protocol)}</td>
          <td>${escapeHtml(record.localAddress)}</td>
          <td>${record.localPort}</td>
          <td>${escapeHtml(record.state)}</td>
          <td>${record.pid}</td>
          <td>${escapeHtml(record.processName)}</td>
          <td><span class="pill ${record.risk === "普通" ? "ok" : "warn"}">${escapeHtml(record.risk)}</span></td>
          <td><button class="icon-action" data-action="kill-port" data-pid="${record.pid}" title="结束进程">${icon(Trash2)}</button></td>
        </tr>
      `,
    )
    .join("");
}

function renderProjectHealth(health: ProjectHealth) {
  const element = document.querySelector<HTMLElement>("#project-health");
  if (!element) return;
  element.innerHTML = `
    <div class="project-summary">
      <strong>${escapeHtml(health.root)}</strong>
      <span>${health.projectTypes.length ? health.projectTypes.join(" / ") : "未识别项目类型"}</span>
    </div>
    <div class="chip-row">${health.signals.map((item) => `<span>${escapeHtml(item)}</span>`).join("")}</div>
    <ul>${health.suggestions.map((item) => `<li>${escapeHtml(item)}</li>`).join("")}</ul>
  `;
}

async function refreshAll() {
  const [snapshot, config, envSnapshot, runtimes, ports] = await Promise.all([
    invoke<AppSnapshot>("app_snapshot"),
    invoke<ConfigView>("load_config"),
    invoke<EnvSnapshot>("env_snapshot"),
    invoke<RuntimeInfo[]>("discover_runtimes"),
    invoke<PortRecord[]>("scan_ports"),
  ]);

  state.snapshot = snapshot;
  state.config = config;
  state.env = envSnapshot;
  state.runtimes = runtimes;
  state.ports = ports;
  renderSnapshot();
  renderEnv();
  renderRuntimes();
  renderPorts();
}

async function runOperation(action: () => Promise<OperationResult | KillResult | ConfigView>, pending: string) {
  showToast(pending);
  try {
    const result = await action();
    if ("message" in result) {
      showToast(result.message);
    } else {
      showToast("操作完成");
    }
    await refreshAll();
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
}

function showToast(message: string, isError = false) {
  const toast = document.querySelector<HTMLElement>("#toast");
  if (!toast) return;
  toast.textContent = message;
  toast.hidden = false;
  toast.classList.toggle("error", isError);
}

function renderProgress(progress: TaskProgress) {
  showToast(`${progress.task}: ${progress.percent}% · ${progress.message}`);
}

function renderNetwork() {
  const element = document.querySelector<HTMLElement>("#network-result");
  if (!element) return;
  const checks = state.network?.checks || [];
  const proxy = state.network?.proxy || [];
  element.innerHTML = checks.length
    ? [
        ...checks.map(
          (check) => `
            <article class="runtime">
              <div><strong>${escapeHtml(check.name)}</strong><span>${check.success ? "正常" : "异常"}</span></div>
              <small>${escapeHtml(check.status)} · ${check.elapsedMs} ms · ${escapeHtml(check.url)}</small>
            </article>
          `,
        ),
        `<article class="runtime"><div><strong>代理</strong><span>${proxy.length} 项</span></div><small>${proxy
          .map(([key, value]) => `${key}=${value || "未设置"}`)
          .map(escapeHtml)
          .join(" · ")}</small></article>`,
      ].join("")
    : `<div class="empty">还没有网络诊断结果</div>`;
}

function renderCache() {
  const element = document.querySelector<HTMLElement>("#cache-list");
  if (!element) return;
  element.innerHTML = state.cache.length
    ? state.cache
        .map(
          (item) => `
            <article class="runtime">
              <div><strong>${escapeHtml(item.name)}</strong><span>${formatBytes(item.size)}</span></div>
              <small>${escapeHtml(item.sha256 || item.path)}</small>
            </article>
          `,
        )
        .join("")
    : `<div class="empty">下载缓存为空</div>`;
}

function formatBytes(size: number) {
  if (size >= 1024 * 1024 * 1024) return `${(size / 1024 / 1024 / 1024).toFixed(2)} GB`;
  if (size >= 1024 * 1024) return `${(size / 1024 / 1024).toFixed(2)} MB`;
  if (size >= 1024) return `${(size / 1024).toFixed(2)} KB`;
  return `${size} B`;
}

function activateView(view: string) {
  document.querySelectorAll(".nav-item").forEach((item) => {
    item.classList.toggle("active", item.getAttribute("data-view") === view);
  });
  document.querySelectorAll(".view").forEach((item) => {
    item.classList.toggle("active", item.id === `view-${view}`);
  });
}

function escapeHtml(value: string) {
  return value.replace(/[&<>"']/g, (char) => {
    const map: Record<string, string> = {
      "&": "&amp;",
      "<": "&lt;",
      ">": "&gt;",
      '"': "&quot;",
      "'": "&#039;",
    };
    return map[char];
  });
}

document.querySelectorAll<HTMLButtonElement>(".nav-item").forEach((button) => {
  button.addEventListener("click", () => activateView(button.dataset.view || "overview"));
});

document.querySelector("#refresh-all")?.addEventListener("click", () => void refreshAll());
document.querySelector("#save-root")?.addEventListener("click", () => {
  const input = document.querySelector<HTMLInputElement>("#root-dir");
  if (!input) return;
  void runOperation(() => invoke<ConfigView>("set_root_dir", { root: input.value }), "正在保存根目录");
});
document.querySelector("#scan-ports")?.addEventListener("click", async () => {
  state.ports = await invoke<PortRecord[]>("scan_ports");
  renderPorts();
});
document.querySelector("#discover-runtimes")?.addEventListener("click", async () => {
  state.runtimes = await invoke<RuntimeInfo[]>("discover_runtimes");
  renderRuntimes();
});
document.querySelector("#install-jdk")?.addEventListener("click", () => {
  const select = document.querySelector<HTMLSelectElement>("#jdk-version");
  if (!select) return;
  void runOperation(
    () => invoke<OperationResult>("install_jdk", { version: select.value }),
    `正在安装 JDK ${select.value}`,
  );
});
document.querySelector("#install-node")?.addEventListener("click", () => {
  const select = document.querySelector<HTMLSelectElement>("#node-version");
  if (!select) return;
  void runOperation(
    () => invoke<OperationResult>("install_node", { version: select.value }),
    `正在安装 Node.js ${select.value}`,
  );
});
document.querySelector("#install-python")?.addEventListener("click", () => {
  const select = document.querySelector<HTMLSelectElement>("#python-version");
  if (!select) return;
  void runOperation(
    () => invoke<OperationResult>("install_python", { version: select.value }),
    `正在安装 Python ${select.value}`,
  );
});
document.querySelector("#install-maven")?.addEventListener("click", () => {
  void runOperation(() => invoke<OperationResult>("install_maven_latest"), "正在安装 Maven 最新版");
});
document.querySelector("#install-gradle")?.addEventListener("click", () => {
  void runOperation(() => invoke<OperationResult>("install_gradle_latest"), "正在安装 Gradle 最新版");
});
document.querySelector("#configure-env")?.addEventListener("click", () => {
  void runOperation(() => invoke<OperationResult>("configure_user_environment"), "正在配置用户环境变量");
});
document.querySelector("#restore-env")?.addEventListener("click", () => {
  void runOperation(() => invoke<OperationResult>("restore_user_environment"), "正在恢复用户环境变量");
});
document.querySelector("#run-network")?.addEventListener("click", async () => {
  showToast("正在执行网络诊断");
  try {
    state.network = await invoke<NetworkDiagnostics>("network_diagnostics");
    renderNetwork();
    showToast("网络诊断完成");
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
document.querySelector("#load-cache")?.addEventListener("click", async () => {
  state.cache = await invoke<CacheEntry[]>("cache_entries", { calculateHash: false });
  renderCache();
});
document.querySelector("#clear-cache")?.addEventListener("click", () => {
  void runOperation(() => invoke<OperationResult>("clear_download_cache"), "正在清理下载缓存");
});
document.querySelector("#run-command")?.addEventListener("click", async () => {
  const command = document.querySelector<HTMLInputElement>("#command-input")?.value || "";
  const cwd = document.querySelector<HTMLInputElement>("#command-cwd")?.value || "";
  const output = document.querySelector<HTMLElement>("#command-output");
  showToast("正在运行命令");
  try {
    const result = await invoke<CommandRunResult>("run_tool_command", {
      command,
      cwd: cwd || null,
    });
    if (output) {
      output.textContent = `退出码 ${result.returnCode} · ${result.elapsedMs} ms\n${result.output}`;
    }
    showToast(result.success ? "命令运行完成" : "命令运行失败", !result.success);
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
document.querySelector("#check-project")?.addEventListener("click", async () => {
  const input = document.querySelector<HTMLInputElement>("#project-path");
  if (!input) return;
  const health = await invoke<ProjectHealth>("project_health", { path: input.value });
  renderProjectHealth(health);
});
document.querySelector("#generate-vscode")?.addEventListener("click", () => {
  const input = document.querySelector<HTMLInputElement>("#project-path");
  if (!input) return;
  void runOperation(
    () => invoke<OperationResult>("generate_vscode_config", { projectPath: input.value }),
    "正在生成 VS Code 配置",
  );
});

document.addEventListener("click", (event) => {
  const button = (event.target as HTMLElement).closest<HTMLButtonElement>("button[data-action]");
  if (!button) return;
  const action = button.dataset.action;
  if (action === "switch-jdk") {
    const version = button.dataset.version || "";
    void runOperation(
      () => invoke<OperationResult>("switch_runtime", { kind: "jdk", version }),
      `正在切换 JDK ${version}`,
    );
  }
  if (action === "uninstall-jdk") {
    const version = button.dataset.version || "";
    void runOperation(
      () => invoke<OperationResult>("uninstall_runtime", { kind: "jdk", version }),
      `正在卸载 JDK ${version}`,
    );
  }
  if (action === "switch-node") {
    const version = button.dataset.version || "";
    void runOperation(
      () => invoke<OperationResult>("switch_runtime", { kind: "node", version }),
      `正在切换 Node.js ${version}`,
    );
  }
  if (action === "uninstall-node") {
    const version = button.dataset.version || "";
    void runOperation(
      () => invoke<OperationResult>("uninstall_runtime", { kind: "node", version }),
      `正在卸载 Node.js ${version}`,
    );
  }
  if (action === "switch-python") {
    const version = button.dataset.version || "";
    void runOperation(
      () => invoke<OperationResult>("switch_runtime", { kind: "python", version }),
      `正在切换 Python ${version}`,
    );
  }
  if (action === "uninstall-python") {
    const version = button.dataset.version || "";
    void runOperation(
      () => invoke<OperationResult>("uninstall_runtime", { kind: "python", version }),
      `正在卸载 Python ${version}`,
    );
  }
  if (action === "switch-build-tool") {
    const kind = button.dataset.kind || "";
    const version = button.dataset.version || "";
    void runOperation(
      () => invoke<OperationResult>("switch_runtime", { kind, version }),
      `正在切换 ${kind} ${version}`,
    );
  }
  if (action === "uninstall-build-tool") {
    const kind = button.dataset.kind || "";
    const version = button.dataset.version || "";
    void runOperation(
      () => invoke<OperationResult>("uninstall_runtime", { kind, version }),
      `正在卸载 ${kind} ${version}`,
    );
  }
  if (action === "kill-port") {
    const pid = Number(button.dataset.pid || 0);
    void runOperation(
      () => invoke<KillResult>("kill_process", { pid, force: false, allowCaution: false }),
      `正在结束 PID ${pid}`,
    );
  }
});

void listen<TaskProgress>("task-progress", (event) => renderProgress(event.payload));
void refreshAll();
