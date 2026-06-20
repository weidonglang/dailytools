import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Activity,
  Boxes,
  Clipboard,
  Cpu,
  Download,
  FileText,
  FolderSearch,
  Gauge,
  Hammer,
  KeyRound,
  Network,
  PackageCheck,
  Play,
  RefreshCw,
  Route,
  Search,
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
    gos: ManagedRuntime[];
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

type PortSortKey = "protocol" | "localAddress" | "localPort" | "state" | "pid" | "processName" | "risk";
type SortDirection = "asc" | "desc";

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

type EnvHealthCheck = {
  name: string;
  status: string;
  detail: string;
};

type ConfigProfile = {
  id: string;
  name: string;
  createdAt: string;
  current: Record<string, string | null>;
  devenvHome?: string;
  javaHome?: string;
  path: string;
};

type DoctorReport = {
  score: number;
  summary: string;
  generatedAt: string;
  checks: Array<{
    id: string;
    title: string;
    category: string;
    status: string;
    severity: string;
    detail: string;
    fixAction?: string;
  }>;
  suggestions: Array<{
    id: string;
    title: string;
    description: string;
    action?: string;
  }>;
};

type PythonAnalysis = {
  currentPython?: PythonToolState;
  currentPip?: PythonToolState;
  launcherOutput: string;
  discoveredPythons: PythonEntry[];
  discoveredPips: PythonEntry[];
  risks: string[];
  recommendations: string[];
  pipRepairCommand: string;
  aliasSettingsCommand: string;
};

type PythonToolState = {
  path: string;
  version: string;
  status: string;
  detail: string;
};

type PythonEntry = {
  path: string;
  source: string;
  version: string;
  current: boolean;
};

type ProjectAnalysis = {
  root: string;
  projectTypes: string[];
  detectedFiles: string[];
  packageManager?: string;
  recommendedRuntime: Array<{
    name: string;
    requirement: string;
    status: string;
  }>;
  actions: Array<{
    id: string;
    title: string;
    command: string;
    description: string;
    safeToRun: boolean;
  }>;
  warnings: string[];
};

type ToolState = {
  name: string;
  installed: boolean;
  version: string;
  path: string;
  detail: string;
};

type ToolchainReport = {
  git: {
    git: ToolState;
    gitBashPath: string;
    userName: string;
    userEmail: string;
    ssh: ToolState;
    sshKeyExists: boolean;
    publicKeyPath: string;
    publicKey: string;
    githubSshStatus: string;
    githubHttpsStatus: string;
    gitLfs: ToolState;
  };
  node: {
    tools: ToolState[];
    npmPrefix: string;
    npmRegistry: string;
    pnpmStorePath: string;
  };
  python: {
    tools: ToolState[];
    pipConfig: string;
    pipIndexUrl: string;
  };
  generatedAt: string;
};

type PlatformReport = {
  go: {
    go: ToolState;
    goroot: string;
    gopath: string;
    goproxy: string;
    gomodcache: string;
  };
  rust: {
    tools: ToolState[];
    defaultToolchain: string;
    installedToolchains: string[];
    msvcBuildTools: string;
    cargoConfigPath: string;
  };
  dotnet: {
    dotnet: ToolState;
    sdks: string[];
    runtimes: string[];
  };
  mirrors: {
    npmRegistry: string;
    pipIndexUrl: string;
    goProxy: string;
    mavenSettingsPath: string;
    mavenSettingsExists: boolean;
    gradleInitPath: string;
    gradleInitExists: boolean;
    cargoConfigPath: string;
    cargoConfigExists: boolean;
  };
  generatedAt: string;
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
          <span>Windows 开发环境管理</span>
        </div>
      </div>
      <nav class="nav">
        <button class="nav-item active" data-view="overview">${icon(Gauge)}<span>总览</span></button>
        <button class="nav-item" data-view="doctor">${icon(Shield)}<span>环境医生</span></button>
        <button class="nav-item" data-view="ports">${icon(Network)}<span>端口</span></button>
        <button class="nav-item" data-view="runtimes">${icon(Terminal)}<span>运行时</span></button>
        <button class="nav-item" data-view="environment">${icon(Route)}<span>环境</span></button>
        <button class="nav-item" data-view="project">${icon(FolderSearch)}<span>项目</span></button>
        <button class="nav-item" data-view="toolchains">${icon(PackageCheck)}<span>工具链</span></button>
        <button class="nav-item" data-view="platforms">${icon(Cpu)}<span>平台/镜像</span></button>
        <button class="nav-item" data-view="toolbox">${icon(Hammer)}<span>工具箱</span></button>
      </nav>
    </aside>
    <section class="workspace">
      <header class="topbar">
        <div>
          <h1>DevEnv Manager</h1>
          <p id="subtitle">诊断、修复、切换和启动项目</p>
        </div>
        <button id="refresh-all" class="primary">${icon(RefreshCw)}<span>刷新</span></button>
      </header>
      <div id="toast" class="toast" hidden></div>
      <div id="task-progress" class="task-progress" hidden>
        <div><strong id="task-progress-title">任务</strong><span id="task-progress-message">等待中</span></div>
        <div class="progress-track"><span id="task-progress-bar"></span></div>
      </div>

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
              <li><span class="dot done"></span> Python / Node / Maven / Gradle 安装和验证</li>
              <li><span class="dot done"></span> 环境医生、Python 冲突分析、项目启动向导</li>
              <li><span class="dot done"></span> Git / Node / Python 开发工具链</li>
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

      <section id="view-doctor" class="view">
        <section class="panel doctor-panel">
          <div class="panel-head">
            <div class="panel-title">${icon(Shield)}<h2>环境医生</h2></div>
            <div class="toolbar compact">
              <button id="run-doctor" class="primary">${icon(Activity)}<span>一键诊断</span></button>
              <button id="export-doctor">${icon(FileText)}<span>导出报告</span></button>
            </div>
          </div>
          <div id="doctor-score" class="doctor-score">
            <strong>--</strong>
            <span>还没有诊断结果</span>
          </div>
          <div id="doctor-suggestions" class="suggestion-list"></div>
          <div id="doctor-checks" class="doctor-checks"></div>
        </section>
      </section>

      <section id="view-ports" class="view">
        <section class="panel">
          <div class="panel-head">
            <div class="panel-title">${icon(Network)}<h2>端口管理</h2></div>
            <button id="scan-ports">${icon(RefreshCw)}<span>扫描</span></button>
          </div>
          <div class="port-tools">
            <div class="search-box">${icon(Search)}<input id="port-search" placeholder="输入 8080、java、web、mysql、pid、进程名..." /></div>
            <div id="port-quick-filters" class="chip-row port-filter-row">
              <button class="filter-chip active" data-port-filter="all">全部</button>
              <button class="filter-chip" data-port-filter="spring">Spring</button>
              <button class="filter-chip" data-port-filter="tomcat">Tomcat</button>
              <button class="filter-chip" data-port-filter="frontend">前端</button>
              <button class="filter-chip" data-port-filter="database">数据库</button>
              <button class="filter-chip" data-port-filter="sensitive">敏感</button>
            </div>
          </div>
          <div class="table-wrap">
            <table>
              <colgroup>
                <col class="col-protocol" />
                <col class="col-address" />
                <col class="col-port" />
                <col class="col-state" />
                <col class="col-pid" />
                <col class="col-process" />
                <col class="col-risk" />
                <col class="col-action" />
              </colgroup>
              <thead>
                <tr>
                  <th><button class="sort-head" data-sort="protocol">协议</button></th>
                  <th><button class="sort-head" data-sort="localAddress">本地地址</button></th>
                  <th><button class="sort-head" data-sort="localPort">端口</button></th>
                  <th><button class="sort-head" data-sort="state">状态</button></th>
                  <th><button class="sort-head" data-sort="pid">PID</button></th>
                  <th><button class="sort-head" data-sort="processName">进程</button></th>
                  <th><button class="sort-head" data-sort="risk">风险</button></th>
                  <th>操作</th>
                </tr>
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
          <div class="panel-head">
            <div class="panel-title">${icon(Activity)}<h2>Python 环境分析</h2></div>
            <button id="analyze-python">${icon(Search)}<span>分析</span></button>
          </div>
          <div id="python-analysis" class="python-analysis"></div>
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
              <button id="check-env-health">${icon(Activity)}<span>检查</span></button>
              <button id="cleanup-path">${icon(Trash2)}<span>清理失效 PATH</span></button>
              <button id="restore-env">${icon(RefreshCw)}<span>恢复</span></button>
            </div>
            <div id="env-list" class="kv-list"></div>
            <div id="env-health" class="runtime-list health-list"></div>
          </section>
          <section class="panel">
            <div class="panel-title">${icon(Shield)}<h2>配置模板</h2></div>
            <div class="form-row">
              <input id="profile-name" placeholder="例如 Java 8 + Python 3.12" />
              <button id="save-profile">${icon(Download)}<span>保存</span></button>
            </div>
            <div id="profile-list" class="runtime-list profile-list"></div>
          </section>
          <section class="panel">
            <div class="panel-title">${icon(Shield)}<h2>PATH 检查</h2></div>
            <div id="path-warnings" class="warning-list"></div>
          </section>
        </div>
      </section>

      <section id="view-toolchains" class="view">
        <section class="panel">
          <div class="panel-head">
            <div class="panel-title">${icon(PackageCheck)}<h2>开发工具链</h2></div>
            <button id="inspect-toolchains" class="primary">${icon(RefreshCw)}<span>全面检查</span></button>
          </div>
        </section>

        <section class="panel toolchain-section">
          <div class="panel-title">${icon(KeyRound)}<h2>Git / GitHub</h2></div>
          <div id="git-toolchain" class="toolchain-content"><div class="empty">点击“全面检查”读取 Git 环境</div></div>
          <div class="form-row toolchain-form">
            <input id="git-user-name" placeholder="Git 用户名" />
            <input id="git-user-email" type="email" placeholder="Git 邮箱" />
            <button id="save-git-identity">${icon(Shield)}<span>保存身份</span></button>
          </div>
          <div class="toolbar">
            <button id="generate-ssh-key">${icon(KeyRound)}<span>生成 ed25519 Key</span></button>
            <button id="test-github-ssh">${icon(Activity)}<span>测试 GitHub SSH</span></button>
            <button id="copy-public-key">${icon(Clipboard)}<span>复制公钥</span></button>
          </div>
        </section>

        <div class="grid two">
          <section class="panel toolchain-section">
            <div class="panel-title">${icon(PackageCheck)}<h2>Node.js 生态</h2></div>
            <div id="node-toolchain" class="toolchain-content"><div class="empty">尚未检查 Node.js 工具链</div></div>
            <div class="toolbar">
              <button data-toolchain-action="corepack_enable">启用 Corepack</button>
              <button data-toolchain-action="npm_install_pnpm">安装 pnpm</button>
              <button data-toolchain-action="npm_install_yarn">安装 Yarn</button>
              <button data-toolchain-action="npm_managed_prefix">使用受管全局目录</button>
            </div>
            <div class="form-row">
              <select id="npm-registry">
                <option value="official">npm 官方源</option>
                <option value="npmmirror">npmmirror</option>
              </select>
              <button id="set-npm-registry">切换 npm 源</button>
            </div>
          </section>

          <section class="panel toolchain-section">
            <div class="panel-title">${icon(PackageCheck)}<h2>Python 生态</h2></div>
            <div id="python-toolchain" class="toolchain-content"><div class="empty">尚未检查 Python 工具链</div></div>
            <div class="toolbar">
              <button data-python-tool="uv">安装 uv</button>
              <button data-python-tool="poetry">安装 Poetry</button>
              <button data-python-tool="virtualenv">安装 virtualenv</button>
            </div>
            <div class="form-row">
              <select id="pip-index">
                <option value="official">PyPI 官方源</option>
                <option value="tsinghua">清华源</option>
                <option value="aliyun">阿里云源</option>
                <option value="ustc">中科大源</option>
              </select>
              <button id="set-pip-index">切换 pip 源</button>
            </div>
          </section>
        </div>
      </section>

      <section id="view-platforms" class="view">
        <section class="panel">
          <div class="panel-head">
            <div class="panel-title">${icon(Cpu)}<h2>平台工具链</h2></div>
            <button id="inspect-platforms" class="primary">${icon(RefreshCw)}<span>全面检查</span></button>
          </div>
        </section>

        <section class="panel platform-section">
          <div class="panel-head">
            <div class="panel-title">${icon(Download)}<h2>Go 管理</h2></div>
            <div class="toolbar compact">
              <select id="go-version">
                <option value="1.22">Go 1.22</option>
                <option value="1.23">Go 1.23</option>
                <option value="1.24">Go 1.24</option>
                <option value="1.25">Go 1.25</option>
                <option value="1.26" selected>Go 1.26</option>
              </select>
              <button id="install-go">${icon(Download)}<span>安装</span></button>
            </div>
          </div>
          <div id="managed-gos" class="runtime-list"></div>
          <div id="go-platform" class="platform-content"><div class="empty">尚未检查 Go 环境</div></div>
          <div class="form-row">
            <select id="go-proxy">
              <option value="official">Go 官方代理</option>
              <option value="goproxy_cn">goproxy.cn</option>
              <option value="direct">仅 direct</option>
            </select>
            <button id="set-go-proxy">切换 GOPROXY</button>
          </div>
        </section>

        <div class="grid two">
          <section class="panel platform-section">
            <div class="panel-title">${icon(Cpu)}<h2>Rust / rustup</h2></div>
            <div id="rust-platform" class="platform-content"><div class="empty">尚未检查 Rust 环境</div></div>
            <div class="toolbar">
              <button id="rust-stable">切换 stable</button>
              <button id="rust-update">更新工具链</button>
              <button data-action="copy-text" data-copy="https://rustup.rs/">${icon(Clipboard)}<span>复制 rustup 地址</span></button>
              <button id="copy-cargo-mirror">${icon(Clipboard)}<span>复制 Cargo 镜像配置</span></button>
            </div>
          </section>

          <section class="panel platform-section">
            <div class="panel-title">${icon(Cpu)}<h2>.NET SDK</h2></div>
            <div id="dotnet-platform" class="platform-content"><div class="empty">尚未检查 .NET SDK</div></div>
            <div class="toolbar">
              <button data-action="copy-text" data-copy="https://dotnet.microsoft.com/download">${icon(Clipboard)}<span>复制下载地址</span></button>
              <button data-action="copy-text" data-copy="dotnet --list-sdks; dotnet --list-runtimes">${icon(Clipboard)}<span>复制检查命令</span></button>
            </div>
          </section>
        </div>

        <section class="panel platform-section">
          <div class="panel-title">${icon(Network)}<h2>镜像加速中心</h2></div>
          <div id="mirror-platform" class="platform-content"><div class="empty">尚未读取镜像配置</div></div>
          <div class="mirror-actions">
            <div class="form-row">
              <select id="maven-mirror">
                <option value="official">Maven 官方源</option>
                <option value="aliyun">阿里云 Maven 镜像</option>
              </select>
              <button id="set-maven-mirror">写入 Maven 配置</button>
              <button id="restore-maven-config" title="恢复最近的 DevEnv Manager 备份">${icon(RefreshCw)}</button>
            </div>
            <div class="form-row">
              <select id="gradle-mirror">
                <option value="official">Gradle 默认源</option>
                <option value="aliyun">阿里云 Maven 镜像</option>
              </select>
              <button id="set-gradle-mirror">写入 Gradle 配置</button>
              <button id="restore-gradle-config" title="恢复最近的 DevEnv Manager 备份">${icon(RefreshCw)}</button>
            </div>
            <button id="open-package-mirrors">管理 npm / pip 源</button>
          </div>
        </section>
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
        <section class="panel runtime-manager danger-panel">
          <div class="panel-title">${icon(Trash2)}<h2>卸载本程序</h2></div>
          <div class="toolbar">
            <button id="self-uninstall" class="danger-button">${icon(Trash2)}<span>启动卸载程序</span></button>
          </div>
          <div class="small-note">会打开 Windows 卸载器并关闭当前程序。</div>
        </section>
      </section>

      <section id="view-project" class="view">
        <section class="panel">
          <div class="panel-title">${icon(FolderSearch)}<h2>项目启动向导</h2></div>
          <div class="form-row">
            <input id="project-path" value="E:\\\\pycode\\\\dailytools" />
            <button id="check-project">${icon(Play)}<span>分析</span></button>
          </div>
          <div class="toolbar project-actions">
            <button id="generate-vscode">${icon(Hammer)}<span>生成 VS Code 配置</span></button>
          </div>
          <div id="project-health" class="project-health"></div>
          <pre id="project-output" class="command-output"></pre>
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
  health: [] as EnvHealthCheck[],
  profiles: [] as ConfigProfile[],
  doctor: null as DoctorReport | null,
  python: null as PythonAnalysis | null,
  project: null as ProjectAnalysis | null,
  toolchains: null as ToolchainReport | null,
  platforms: null as PlatformReport | null,
};

const portState = {
  sortKey: "localPort" as PortSortKey,
  sortDirection: "asc" as SortDirection,
  query: "",
  quickFilter: "all",
};

const commonPorts: Array<{ key: string; label: string; ports: number[]; keywords: string[] }> = [
  { key: "spring", label: "Spring", ports: [8080, 8081, 8082, 8888, 8761], keywords: ["java", "spring"] },
  { key: "tomcat", label: "Tomcat", ports: [8080, 8005, 8009, 8443], keywords: ["tomcat", "java"] },
  { key: "frontend", label: "前端", ports: [3000, 4173, 5173, 5174, 8080], keywords: ["node", "vite", "npm"] },
  { key: "database", label: "数据库", ports: [3306, 5432, 6379, 27017, 9200], keywords: ["mysql", "postgres", "redis", "mongo", "elastic"] },
];

const portAliases: Record<string, string[]> = {
  boot: ["spring"],
  springboot: ["spring"],
  java: ["spring", "tomcat"],
  web: ["spring", "tomcat", "frontend"],
  vite: ["frontend"],
  vue: ["frontend"],
  react: ["frontend"],
  mysql: ["3306", "database"],
  redis: ["6379", "database"],
  postgres: ["5432", "database"],
  postgresql: ["5432", "database"],
  mongo: ["27017", "database"],
  elastic: ["9200", "database"],
  es: ["9200", "database"],
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
    ? state.env.pathWarnings
        .map((item) => {
          const kind = item.startsWith("托管 PATH") ? "pending" : item.startsWith("重复 PATH") ? "duplicate" : "invalid";
          return `<div class="warning ${kind}">${escapeHtml(item)}</div>`;
        })
        .join("")
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
              ${canUninstallExternal(runtime) ? `<div class="row-actions"><button data-action="uninstall-external-runtime" data-kind="${escapeHtml(runtime.kind)}" data-executable="${escapeHtml(runtime.executable)}">${icon(Trash2)}<span>系统卸载</span></button></div>` : ""}
            </article>
          `,
        )
        .join("")
    : `<div class="empty">还没有发现运行时</div>`;
  renderManagedJdks();
  renderManagedNodes();
  renderManagedPythons();
  renderManagedBuildTools();
  renderManagedGos();
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
                <button data-action="switch-jdk" data-version="${escapeHtml(jdk.version)}" data-path="${escapeHtml(jdk.path)}">${icon(RefreshCw)}<span>切换</span></button>
                <button data-action="uninstall-jdk" data-version="${escapeHtml(jdk.version)}" data-path="${escapeHtml(jdk.path)}">${icon(Trash2)}<span>卸载</span></button>
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
                <button data-action="switch-node" data-version="${escapeHtml(node.version)}" data-path="${escapeHtml(node.path)}">${icon(RefreshCw)}<span>切换</span></button>
                <button data-action="uninstall-node" data-version="${escapeHtml(node.version)}" data-path="${escapeHtml(node.path)}">${icon(Trash2)}<span>卸载</span></button>
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
                <button data-action="switch-python" data-version="${escapeHtml(python.version)}" data-path="${escapeHtml(python.path)}">${icon(RefreshCw)}<span>切换</span></button>
                <button data-action="uninstall-python" data-version="${escapeHtml(python.version)}" data-path="${escapeHtml(python.path)}">${icon(Trash2)}<span>卸载</span></button>
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
                <button data-action="switch-build-tool" data-kind="${item.kind}" data-version="${escapeHtml(item.version)}" data-path="${escapeHtml(item.path)}">${icon(RefreshCw)}<span>切换</span></button>
                <button data-action="uninstall-build-tool" data-kind="${item.kind}" data-version="${escapeHtml(item.version)}" data-path="${escapeHtml(item.path)}">${icon(Trash2)}<span>卸载</span></button>
              </div>
            </article>
          `,
        )
        .join("")
    : `<div class="empty">还没有安装受管 Maven 或 Gradle</div>`;
}

function renderManagedGos() {
  const element = document.querySelector<HTMLElement>("#managed-gos");
  if (!element) return;
  const gos = state.config?.installed.gos || [];
  const current = state.config?.installed.current.go;
  element.innerHTML = gos.length
    ? gos
        .map(
          (go) => `
            <article class="runtime managed-runtime">
              <div>
                <strong>Go ${escapeHtml(go.version)}${current === go.version ? " · 当前" : ""}</strong>
                <span>${escapeHtml(go.detail || "")}</span>
              </div>
              <small>${escapeHtml(go.path)}</small>
              <div class="row-actions">
                <button data-action="switch-go" data-version="${escapeHtml(go.version)}" data-path="${escapeHtml(go.path)}">${icon(RefreshCw)}<span>切换</span></button>
                <button data-action="uninstall-go" data-version="${escapeHtml(go.version)}" data-path="${escapeHtml(go.path)}">${icon(Trash2)}<span>卸载</span></button>
              </div>
            </article>
          `,
        )
        .join("")
    : `<div class="empty">还没有安装受管 Go</div>`;
}

function renderPorts() {
  const visible = sortedPorts(filteredPorts(state.ports));
  setText("metric-ports", visible.length);
  const body = document.querySelector<HTMLElement>("#ports-body");
  if (!body) return;
  body.innerHTML = visible
    .slice(0, 250)
    .map(
      (record) => {
        const hint = portHint(record);
        return `
        <tr>
          <td>${escapeHtml(record.protocol)}</td>
          <td>${escapeHtml(record.localAddress)}</td>
          <td><strong>${record.localPort}</strong>${hint ? `<span class="port-hint">${escapeHtml(hint)}</span>` : ""}</td>
          <td>${escapeHtml(record.state)}</td>
          <td>${record.pid}</td>
          <td>${escapeHtml(record.processName)}</td>
          <td><span class="pill ${record.risk === "普通" ? "ok" : "warn"}">${escapeHtml(record.risk)}</span></td>
          <td><button class="icon-action" data-action="kill-port" data-pid="${record.pid}" title="结束进程">${icon(Trash2)}</button></td>
        </tr>
      `;
      },
    )
    .join("");
  updateSortHeaders();
}

function canUninstallExternal(runtime: RuntimeInfo) {
  const source = runtime.source.toLowerCase();
  return (
    !source.includes("devenv") &&
    ["Java", "Python", "Node.js", "Maven", "Gradle", "Go"].includes(runtime.kind)
  );
}

function renderHealth() {
  const element = document.querySelector<HTMLElement>("#env-health");
  if (!element) return;
  element.innerHTML = state.health.length
    ? state.health
        .map(
          (item) => `
            <article class="runtime health-item ${item.status === "正常" ? "ok" : "warn"}">
              <div><strong>${escapeHtml(item.name)}</strong><span>${escapeHtml(item.status)}</span></div>
              <small>${escapeHtml(item.detail)}</small>
            </article>
          `,
        )
        .join("")
    : `<div class="empty">还没有环境健康检查结果</div>`;
}

function renderProfiles() {
  const element = document.querySelector<HTMLElement>("#profile-list");
  if (!element) return;
  element.innerHTML = state.profiles.length
    ? state.profiles
        .map((profile) => {
          const current = Object.entries(profile.current)
            .filter(([, value]) => value)
            .map(([key, value]) => `${key} ${value}`)
            .join(" · ");
          return `
            <article class="runtime profile-item">
              <div><strong>${escapeHtml(profile.name)}</strong><span>${escapeHtml(profile.createdAt)}</span></div>
              <small>${escapeHtml(current || "仅保存环境变量")}</small>
              <div class="row-actions">
                <button data-action="apply-profile" data-id="${escapeHtml(profile.id)}">${icon(RefreshCw)}<span>应用</span></button>
                <button data-action="delete-profile" data-id="${escapeHtml(profile.id)}">${icon(Trash2)}<span>删除</span></button>
              </div>
            </article>
          `;
        })
        .join("")
    : `<div class="empty">还没有保存配置模板</div>`;
}

function renderDoctor() {
  const score = document.querySelector<HTMLElement>("#doctor-score");
  const checks = document.querySelector<HTMLElement>("#doctor-checks");
  const suggestions = document.querySelector<HTMLElement>("#doctor-suggestions");
  if (!score || !checks || !suggestions) return;
  const report = state.doctor;
  if (!report) {
    score.innerHTML = `<strong>--</strong><span>点击“一键诊断”生成环境评分</span>`;
    checks.innerHTML = `<div class="empty">还没有诊断结果</div>`;
    suggestions.innerHTML = "";
    return;
  }
  const warningCount = report.checks.filter((item) => item.severity !== "info" || item.status !== "正常").length;
  score.innerHTML = `<strong>${report.score}</strong><span>${escapeHtml(report.summary)} · ${warningCount} 项需要关注</span>`;
  suggestions.innerHTML = report.suggestions
    .map(
      (item) => `
        <article class="suggestion">
          <div><strong>${escapeHtml(item.title)}</strong><span>${escapeHtml(item.description)}</span></div>
          ${item.action ? `<button data-action="doctor-fix" data-fix="${escapeHtml(item.action)}">${doctorActionLabel(item.action)}</button>` : ""}
        </article>
      `,
    )
    .join("");
  checks.innerHTML = report.checks
    .map(
      (item) => `
        <article class="doctor-check ${escapeHtml(item.severity)}">
          <div>
            <strong>${escapeHtml(item.title)}</strong>
            <span>${escapeHtml(item.category)} · ${escapeHtml(item.status)}</span>
          </div>
          <small>${escapeHtml(item.detail || "无详情")}</small>
          ${item.fixAction ? `<button data-action="doctor-fix" data-fix="${escapeHtml(item.fixAction)}">${doctorActionLabel(item.fixAction)}</button>` : ""}
        </article>
      `,
    )
    .join("");
}

function doctorActionLabel(action: string) {
  const labels: Record<string, string> = {
    cleanup_path: "清理 PATH",
    configure_env: "配置环境",
    discover_runtimes: "刷新运行时",
    python_analysis: "查看 Python",
    export_report: "导出报告",
    network: "网络诊断",
    ports: "查看端口",
    cache: "查看缓存",
    toolchains: "查看工具链",
    platforms: "查看平台",
    copy_fix_command: "复制建议",
  };
  return labels[action] || "处理";
}

function renderPythonAnalysis() {
  const element = document.querySelector<HTMLElement>("#python-analysis");
  if (!element) return;
  const analysis = state.python;
  if (!analysis) {
    element.innerHTML = `<div class="empty">点击“分析”查看 python、pip、py launcher 和 Microsoft Store 别名风险</div>`;
    return;
  }
  const currentPython = analysis.currentPython
    ? `<article class="runtime">
        <div><strong>默认 python</strong><span>${escapeHtml(analysis.currentPython.status)}</span></div>
        <small>${escapeHtml(analysis.currentPython.version)} · ${escapeHtml(analysis.currentPython.path)}</small>
      </article>`
    : `<article class="runtime warn"><div><strong>默认 python</strong><span>未发现</span></div><small>PATH 上没有可用 python</small></article>`;
  const currentPip = analysis.currentPip
    ? `<article class="runtime ${analysis.currentPip.status === "正常" ? "" : "warn"}">
        <div><strong>默认 pip</strong><span>${escapeHtml(analysis.currentPip.status)}</span></div>
        <small>${escapeHtml(analysis.currentPip.version)} · ${escapeHtml(analysis.currentPip.path)}</small>
      </article>`
    : `<article class="runtime warn"><div><strong>默认 pip</strong><span>未发现</span></div><small>建议使用 python -m ensurepip --upgrade</small></article>`;

  element.innerHTML = `
    <div class="runtime-list">${currentPython}${currentPip}</div>
    <div class="chip-row">${analysis.risks.map((item) => `<span>${escapeHtml(item)}</span>`).join("")}</div>
    <div class="toolbar compact">
      <button data-action="copy-text" data-copy="${escapeHtml(analysis.pipRepairCommand)}">${icon(Clipboard)}<span>复制 pip 修复命令</span></button>
      <button data-action="copy-text" data-copy="${escapeHtml(analysis.aliasSettingsCommand)}">${icon(Clipboard)}<span>复制别名设置命令</span></button>
    </div>
    <div class="grid two compact-grid">
      <section>
        <h3>发现的 Python</h3>
        <div class="runtime-list">
          ${analysis.discoveredPythons
            .map(
              (item) => `
                <article class="runtime">
                  <div><strong>${escapeHtml(item.version)}</strong><span>${escapeHtml(item.source)}${item.current ? " · 默认" : ""}</span></div>
                  <small>${escapeHtml(item.path)}</small>
                </article>
              `,
            )
            .join("") || `<div class="empty">没有发现 Python</div>`}
        </div>
      </section>
      <section>
        <h3>发现的 pip / py launcher</h3>
        <div class="runtime-list">
          ${analysis.discoveredPips
            .map(
              (item) => `
                <article class="runtime">
                  <div><strong>${escapeHtml(item.version)}</strong><span>${escapeHtml(item.source)}${item.current ? " · 默认" : ""}</span></div>
                  <small>${escapeHtml(item.path)}</small>
                </article>
              `,
            )
            .join("") || `<div class="empty">没有发现 pip</div>`}
        </div>
        <pre class="command-output">${escapeHtml(analysis.launcherOutput)}</pre>
      </section>
    </div>
    <ul>${analysis.recommendations.map((item) => `<li>${escapeHtml(item)}</li>`).join("")}</ul>
  `;
}

function renderToolStates(items: ToolState[]) {
  return items
    .map(
      (item) => `
        <article class="runtime tool-state ${item.installed ? "ok" : "warn"}">
          <div><strong>${escapeHtml(item.name)}</strong><span>${item.installed ? "可用" : "缺失"}</span></div>
          <small>${escapeHtml(item.version)}${item.path ? ` · ${escapeHtml(item.path)}` : ""}</small>
        </article>
      `,
    )
    .join("");
}

function renderToolchains() {
  const git = document.querySelector<HTMLElement>("#git-toolchain");
  const node = document.querySelector<HTMLElement>("#node-toolchain");
  const python = document.querySelector<HTMLElement>("#python-toolchain");
  const report = state.toolchains;
  if (!git || !node || !python || !report) return;

  git.innerHTML = `
    <div class="tool-state-grid">${renderToolStates([report.git.git, report.git.ssh, report.git.gitLfs])}</div>
    <div class="kv-list toolchain-kv">
      <div><span>Git Bash</span><strong>${escapeHtml(report.git.gitBashPath || "未发现")}</strong></div>
      <div><span>GitHub HTTPS</span><strong>${escapeHtml(report.git.githubHttpsStatus)}</strong></div>
      <div><span>GitHub SSH</span><strong>${escapeHtml(report.git.githubSshStatus)}</strong></div>
      <div><span>SSH 公钥</span><strong>${escapeHtml(report.git.sshKeyExists ? report.git.publicKeyPath : "未生成")}</strong></div>
    </div>
  `;
  const nameInput = document.querySelector<HTMLInputElement>("#git-user-name");
  const emailInput = document.querySelector<HTMLInputElement>("#git-user-email");
  if (nameInput && !nameInput.value) nameInput.value = report.git.userName;
  if (emailInput && !emailInput.value) emailInput.value = report.git.userEmail;

  node.innerHTML = `
    <div class="tool-state-grid">${renderToolStates(report.node.tools)}</div>
    <div class="kv-list toolchain-kv">
      <div><span>npm registry</span><strong>${escapeHtml(report.node.npmRegistry || "未读取")}</strong></div>
      <div><span>npm 全局目录</span><strong>${escapeHtml(report.node.npmPrefix || "未读取")}</strong></div>
      <div><span>pnpm store</span><strong>${escapeHtml(report.node.pnpmStorePath || "未读取")}</strong></div>
    </div>
  `;
  python.innerHTML = `
    <div class="tool-state-grid">${renderToolStates(report.python.tools)}</div>
    <div class="kv-list toolchain-kv">
      <div><span>pip index-url</span><strong>${escapeHtml(report.python.pipIndexUrl)}</strong></div>
    </div>
    <pre class="command-output compact-output">${escapeHtml(report.python.pipConfig || "pip 没有返回额外配置")}</pre>
  `;
}

async function inspectToolchains(message = "正在检查开发工具链") {
  showToast(message);
  try {
    state.toolchains = await invoke<ToolchainReport>("inspect_toolchains");
    renderToolchains();
    showToast("工具链检查完成");
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
}

async function runToolchainAction(action: string, value: string | null = null, secondary: string | null = null) {
  showToast("正在执行工具链操作");
  try {
    const result = await invoke<OperationResult>("run_toolchain_action", { action, value, secondary });
    showToast(result.message);
    await inspectToolchains("正在验证操作结果");
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
}

function renderPlatforms() {
  const go = document.querySelector<HTMLElement>("#go-platform");
  const rust = document.querySelector<HTMLElement>("#rust-platform");
  const dotnet = document.querySelector<HTMLElement>("#dotnet-platform");
  const mirrors = document.querySelector<HTMLElement>("#mirror-platform");
  const report = state.platforms;
  if (!go || !rust || !dotnet || !mirrors || !report) return;

  go.innerHTML = `
    <div class="tool-state-grid">${renderToolStates([report.go.go])}</div>
    <div class="kv-list toolchain-kv">
      <div><span>GOROOT</span><strong>${escapeHtml(report.go.goroot || "未读取")}</strong></div>
      <div><span>GOPATH</span><strong>${escapeHtml(report.go.gopath || "未读取")}</strong></div>
      <div><span>GOPROXY</span><strong>${escapeHtml(report.go.goproxy || "未读取")}</strong></div>
      <div><span>GOMODCACHE</span><strong>${escapeHtml(report.go.gomodcache || "未读取")}</strong></div>
    </div>
  `;
  rust.innerHTML = `
    <div class="tool-state-grid">${renderToolStates(report.rust.tools)}</div>
    <div class="kv-list toolchain-kv">
      <div><span>默认工具链</span><strong>${escapeHtml(report.rust.defaultToolchain || "未设置")}</strong></div>
      <div><span>MSVC Build Tools</span><strong>${escapeHtml(report.rust.msvcBuildTools)}</strong></div>
      <div><span>Cargo 配置</span><strong>${escapeHtml(report.rust.cargoConfigPath)}</strong></div>
    </div>
    <div class="chip-row">${report.rust.installedToolchains.map((item) => `<span>${escapeHtml(item)}</span>`).join("") || ""}</div>
  `;
  dotnet.innerHTML = `
    <div class="tool-state-grid">${renderToolStates([report.dotnet.dotnet])}</div>
    <div class="platform-columns">
      <div><h3>SDK</h3><pre class="command-output compact-output">${escapeHtml(report.dotnet.sdks.join("\n") || "未发现 SDK")}</pre></div>
      <div><h3>Runtime</h3><pre class="command-output compact-output">${escapeHtml(report.dotnet.runtimes.join("\n") || "未发现 Runtime")}</pre></div>
    </div>
  `;
  mirrors.innerHTML = `
    <div class="kv-list toolchain-kv">
      <div><span>npm registry</span><strong>${escapeHtml(report.mirrors.npmRegistry || "未读取")}</strong></div>
      <div><span>pip index-url</span><strong>${escapeHtml(report.mirrors.pipIndexUrl)}</strong></div>
      <div><span>GOPROXY</span><strong>${escapeHtml(report.mirrors.goProxy || "未读取")}</strong></div>
      <div><span>Maven settings.xml</span><strong>${escapeHtml(report.mirrors.mavenSettingsPath)} · ${report.mirrors.mavenSettingsExists ? "已存在" : "未创建"}</strong></div>
      <div><span>Gradle init.gradle</span><strong>${escapeHtml(report.mirrors.gradleInitPath)} · ${report.mirrors.gradleInitExists ? "已存在" : "未创建"}</strong></div>
      <div><span>Cargo config.toml</span><strong>${escapeHtml(report.mirrors.cargoConfigPath)} · ${report.mirrors.cargoConfigExists ? "已存在" : "未创建"}</strong></div>
    </div>
  `;
}

async function inspectPlatforms(message = "正在检查平台工具链") {
  showToast(message);
  try {
    state.platforms = await invoke<PlatformReport>("inspect_platform_toolchains");
    renderPlatforms();
    showToast("平台工具链检查完成");
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
}

async function runPlatformAction(action: string, value: string | null = null) {
  showToast("正在执行平台工具链操作");
  try {
    const result = await invoke<OperationResult>("run_platform_action", { action, value });
    showToast(result.message);
    await inspectPlatforms("正在验证操作结果");
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
}

function filteredPorts(records: PortRecord[]) {
  const terms = expandPortQuery(portState.query);
  return records.filter((record) => {
    const hint = portHint(record).toLowerCase();
    const text = [
      record.protocol,
      record.localAddress,
      record.localPort,
      record.state,
      record.pid,
      record.processName,
      record.risk,
      hint,
    ]
      .join(" ")
      .toLowerCase();
    const queryMatch = terms.length === 0 || terms.every((term) => text.includes(term));
    const quickMatch =
      portState.quickFilter === "all" ||
      (portState.quickFilter === "sensitive" && record.risk !== "普通") ||
      commonPorts.some(
        (item) =>
          item.key === portState.quickFilter &&
          (item.ports.includes(record.localPort) ||
            item.keywords.some((keyword) => record.processName.toLowerCase().includes(keyword))),
      );
    return queryMatch && quickMatch;
  });
}

function expandPortQuery(query: string) {
  return query
    .trim()
    .toLowerCase()
    .split(/[\s,;，；]+/)
    .filter(Boolean)
    .flatMap((term) => portAliases[term] || [term]);
}

function sortedPorts(records: PortRecord[]) {
  return records.slice().sort((a, b) => {
    const left = a[portState.sortKey];
    const right = b[portState.sortKey];
    const result =
      typeof left === "number" && typeof right === "number"
        ? left - right
        : String(left).localeCompare(String(right), "zh-Hans-CN", { numeric: true, sensitivity: "base" });
    return portState.sortDirection === "asc" ? result : -result;
  });
}

function portHint(record: PortRecord) {
  const exact = commonPorts.filter((item) => item.ports.includes(record.localPort)).map((item) => item.label);
  if (record.risk !== "普通") exact.push(record.risk);
  return [...new Set(exact)].join(" / ");
}

function updateSortHeaders() {
  document.querySelectorAll<HTMLButtonElement>(".sort-head").forEach((button) => {
    const key = button.dataset.sort;
    const active = key === portState.sortKey;
    button.classList.toggle("active", active);
    button.dataset.direction = active ? portState.sortDirection : "";
  });
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

function renderProjectAnalysis(analysis: ProjectAnalysis) {
  const element = document.querySelector<HTMLElement>("#project-health");
  if (!element) return;
  state.project = analysis;
  element.innerHTML = `
    <div class="project-summary">
      <strong>${escapeHtml(analysis.root)}</strong>
      <span>${analysis.projectTypes.length ? analysis.projectTypes.join(" / ") : "未识别项目类型"}</span>
    </div>
    <div class="chip-row">${analysis.detectedFiles.map((item) => `<span>${escapeHtml(item)}</span>`).join("")}</div>
    <div class="grid two compact-grid">
      <section>
        <h3>推荐环境</h3>
        <div class="runtime-list">
          ${analysis.recommendedRuntime
            .map(
              (item) => `
                <article class="runtime">
                  <div><strong>${escapeHtml(item.name)}</strong><span>${escapeHtml(item.status)}</span></div>
                  <small>${escapeHtml(item.requirement)}</small>
                </article>
              `,
            )
            .join("") || `<div class="empty">没有特殊运行时要求</div>`}
        </div>
      </section>
      <section>
        <h3>建议操作</h3>
        <div class="runtime-list">
          ${analysis.actions
            .map(
              (item) => `
                <article class="runtime project-action-item">
                  <div><strong>${escapeHtml(item.title)}</strong><span>${escapeHtml(item.command)}</span></div>
                  <small>${escapeHtml(item.description)}</small>
                  <button data-action="project-run" data-project-action="${escapeHtml(item.id)}">${item.id === "copy_commands" ? icon(Clipboard) : icon(Play)}<span>${item.id === "copy_commands" ? "复制" : "运行"}</span></button>
                </article>
              `,
            )
            .join("")}
        </div>
      </section>
    </div>
    ${analysis.warnings.length ? `<ul>${analysis.warnings.map((item) => `<li>${escapeHtml(item)}</li>`).join("")}</ul>` : ""}
  `;
}

async function refreshBase() {
  const [snapshot, config, envSnapshot, profiles] = await Promise.all([
    invoke<AppSnapshot>("app_snapshot"),
    invoke<ConfigView>("load_config"),
    invoke<EnvSnapshot>("env_snapshot"),
    invoke<ConfigProfile[]>("list_config_profiles"),
  ]);

  state.snapshot = snapshot;
  state.config = config;
  state.env = envSnapshot;
  state.profiles = profiles;
  renderSnapshot();
  renderEnv();
  renderHealth();
  renderProfiles();
  renderDoctor();
  renderPythonAnalysis();
  renderRuntimes();
  renderPorts();
}

async function refreshRuntimeAndPorts(silent = false) {
  try {
    const [runtimes, ports] = await Promise.all([
      invoke<RuntimeInfo[]>("discover_runtimes"),
      invoke<PortRecord[]>("scan_ports"),
    ]);
    state.runtimes = runtimes;
    state.ports = ports;
    renderRuntimes();
    renderPorts();
  } catch (error) {
    if (!silent) {
      showToast(error instanceof Error ? error.message : String(error), true);
    }
  }
}

async function refreshAll(deep = false) {
  await refreshBase();
  if (deep) {
    await refreshRuntimeAndPorts();
  }
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
    await refreshBase();
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
}

async function runRuntimeOperation(
  action: () => Promise<OperationResult | KillResult | ConfigView>,
  pending: string,
  focus: string,
) {
  showToast(pending);
  try {
    const result = await action();
    const message = "message" in result ? result.message : "操作完成";
    showToast(`${message}，正在验证`);
    await refreshBase();
    const [health, runtimes] = await Promise.all([
      invoke<EnvHealthCheck[]>("environment_health"),
      invoke<RuntimeInfo[]>("discover_runtimes"),
    ]);
    state.health = health;
    state.runtimes = runtimes;
    renderHealth();
    renderRuntimes();
    const check = health.find((item) => item.name.toLowerCase() === focus.toLowerCase());
    if (check && check.status !== "正常") {
      showToast(`${message}；${focus} 验证结果：${check.status}，${check.detail}`, true);
    } else {
      showToast(`${message}；${focus} 验证通过`);
    }
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
}

async function copyText(text: string) {
  try {
    await navigator.clipboard.writeText(text);
    showToast("已复制到剪贴板");
  } catch {
    showToast(text);
  }
}

async function runDoctorAction(action: string) {
  if (action === "cleanup_path") {
    await runOperation(() => invoke<OperationResult>("cleanup_path_entries"), "正在清理 PATH");
    return;
  }
  if (action === "configure_env") {
    await runOperation(() => invoke<OperationResult>("configure_user_environment"), "正在配置用户环境变量");
    return;
  }
  if (action === "discover_runtimes") {
    state.runtimes = await invoke<RuntimeInfo[]>("discover_runtimes");
    renderRuntimes();
    activateView("runtimes");
    showToast("运行时刷新完成");
    return;
  }
  if (action === "python_analysis") {
    activateView("runtimes");
    state.python = await invoke<PythonAnalysis>("analyze_python_environment");
    renderPythonAnalysis();
    showToast("Python 环境分析完成");
    return;
  }
  if (action === "export_report") {
    if (!state.doctor) {
      state.doctor = await invoke<DoctorReport>("run_doctor");
      renderDoctor();
    }
    const report = state.doctor!;
    await runOperation(() => invoke<OperationResult>("export_doctor_report", { report }), "正在导出诊断报告");
    return;
  }
  if (action === "network") {
    activateView("toolbox");
    state.network = await invoke<NetworkDiagnostics>("network_diagnostics");
    renderNetwork();
    showToast("网络诊断完成");
    return;
  }
  if (action === "ports") {
    activateView("ports");
    state.ports = await invoke<PortRecord[]>("scan_ports");
    renderPorts();
    showToast("端口扫描完成");
    return;
  }
  if (action === "cache") {
    activateView("toolbox");
    state.cache = await invoke<CacheEntry[]>("cache_entries", { calculateHash: false });
    renderCache();
    showToast("缓存列表已刷新");
    return;
  }
  if (action === "toolchains") {
    activateView("toolchains");
    await inspectToolchains();
    return;
  }
  if (action === "platforms") {
    activateView("platforms");
    await inspectPlatforms();
    return;
  }
  if (action === "copy_fix_command") {
    await copyText("devenv doctor");
    return;
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
  const box = document.querySelector<HTMLElement>("#task-progress");
  const title = document.querySelector<HTMLElement>("#task-progress-title");
  const message = document.querySelector<HTMLElement>("#task-progress-message");
  const bar = document.querySelector<HTMLElement>("#task-progress-bar");
  if (!box || !title || !message || !bar) return;
  box.hidden = false;
  title.textContent = `${progress.task} · ${progress.percent}%`;
  message.textContent = progress.message;
  bar.style.width = `${Math.max(0, Math.min(100, progress.percent))}%`;
  if (progress.percent >= 100) {
    window.setTimeout(() => {
      box.hidden = true;
      bar.style.width = "0%";
    }, 2500);
  }
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

document.querySelector("#refresh-all")?.addEventListener("click", () => void refreshAll(true));
document.querySelector("#run-doctor")?.addEventListener("click", async () => {
  showToast("环境医生正在诊断");
  try {
    state.doctor = await invoke<DoctorReport>("run_doctor");
    renderDoctor();
    showToast("环境诊断完成");
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
document.querySelector("#export-doctor")?.addEventListener("click", async () => {
  try {
    if (!state.doctor) {
      state.doctor = await invoke<DoctorReport>("run_doctor");
      renderDoctor();
    }
    const report = state.doctor!;
    await runOperation(
      () => invoke<OperationResult>("export_doctor_report", { report }),
      "正在导出诊断报告",
    );
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
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
  void runRuntimeOperation(
    () => invoke<OperationResult>("install_jdk", { version: select.value }),
    `正在安装 JDK ${select.value}`,
    "JDK",
  );
});
document.querySelector("#install-node")?.addEventListener("click", () => {
  const select = document.querySelector<HTMLSelectElement>("#node-version");
  if (!select) return;
  void runRuntimeOperation(
    () => invoke<OperationResult>("install_node", { version: select.value }),
    `正在安装 Node.js ${select.value}`,
    "Node.js",
  );
});
document.querySelector("#install-go")?.addEventListener("click", () => {
  const select = document.querySelector<HTMLSelectElement>("#go-version");
  if (!select) return;
  void runRuntimeOperation(
    () => invoke<OperationResult>("install_go", { version: select.value }),
    `正在安装 Go ${select.value}`,
    "Go",
  );
});
document.querySelector("#install-python")?.addEventListener("click", () => {
  const select = document.querySelector<HTMLSelectElement>("#python-version");
  if (!select) return;
  void runRuntimeOperation(
    () => invoke<OperationResult>("install_python", { version: select.value }),
    `正在安装 Python ${select.value}`,
    "Python",
  );
});
document.querySelector("#install-maven")?.addEventListener("click", () => {
  void runRuntimeOperation(() => invoke<OperationResult>("install_maven_latest"), "正在安装 Maven 最新版", "Maven");
});
document.querySelector("#install-gradle")?.addEventListener("click", () => {
  void runRuntimeOperation(() => invoke<OperationResult>("install_gradle_latest"), "正在安装 Gradle 最新版", "Gradle");
});
document.querySelector("#analyze-python")?.addEventListener("click", async () => {
  showToast("正在分析 Python 环境");
  try {
    state.python = await invoke<PythonAnalysis>("analyze_python_environment");
    renderPythonAnalysis();
    showToast("Python 环境分析完成");
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
document.querySelector("#inspect-toolchains")?.addEventListener("click", () => void inspectToolchains());
document.querySelector("#inspect-platforms")?.addEventListener("click", () => void inspectPlatforms());
document.querySelector("#set-go-proxy")?.addEventListener("click", () => {
  const value = document.querySelector<HTMLSelectElement>("#go-proxy")?.value || "official";
  void runPlatformAction("go_proxy", value);
});
document.querySelector("#rust-stable")?.addEventListener("click", () => {
  void runPlatformAction("rust_default_stable");
});
document.querySelector("#rust-update")?.addEventListener("click", () => {
  if (!window.confirm("rustup 将联网更新当前用户安装的 Rust 工具链，可能需要一些时间。确定继续吗？")) return;
  void runPlatformAction("rust_update");
});
document.querySelector("#copy-cargo-mirror")?.addEventListener("click", () => {
  void copyText(`[source.crates-io]\nreplace-with = "rsproxy-sparse"\n\n[source.rsproxy-sparse]\nregistry = "sparse+https://rsproxy.cn/index/"`);
});
document.querySelector("#set-maven-mirror")?.addEventListener("click", () => {
  const value = document.querySelector<HTMLSelectElement>("#maven-mirror")?.value || "official";
  const path = state.platforms?.mirrors.mavenSettingsPath || "%USERPROFILE%\\.m2\\settings.xml";
  if (!window.confirm(`将写入 ${path}。若文件已存在，会先创建带时间戳的备份。确定继续吗？`)) return;
  void runPlatformAction("maven_mirror", value);
});
document.querySelector("#set-gradle-mirror")?.addEventListener("click", () => {
  const value = document.querySelector<HTMLSelectElement>("#gradle-mirror")?.value || "official";
  const path = state.platforms?.mirrors.gradleInitPath || "%USERPROFILE%\\.gradle\\init.gradle";
  if (!window.confirm(`将写入 ${path}。若文件已存在，会先创建带时间戳的备份。确定继续吗？`)) return;
  void runPlatformAction("gradle_mirror", value);
});
document.querySelector("#restore-maven-config")?.addEventListener("click", () => {
  if (!window.confirm("将恢复最近一次 DevEnv Manager 备份的 Maven 配置，并保留当前配置备份。确定继续吗？")) return;
  void runPlatformAction("restore_maven_config");
});
document.querySelector("#restore-gradle-config")?.addEventListener("click", () => {
  if (!window.confirm("将恢复最近一次 DevEnv Manager 备份的 Gradle 配置，并保留当前配置备份。确定继续吗？")) return;
  void runPlatformAction("restore_gradle_config");
});
document.querySelector("#open-package-mirrors")?.addEventListener("click", () => {
  activateView("toolchains");
  if (!state.toolchains) void inspectToolchains();
});
document.querySelector("#save-git-identity")?.addEventListener("click", () => {
  const name = document.querySelector<HTMLInputElement>("#git-user-name")?.value.trim() || "";
  const email = document.querySelector<HTMLInputElement>("#git-user-email")?.value.trim() || "";
  if (!name || !email) {
    showToast("请填写 Git 用户名和邮箱", true);
    return;
  }
  void runToolchainAction("git_identity", name, email);
});
document.querySelector("#generate-ssh-key")?.addEventListener("click", () => {
  const email = document.querySelector<HTMLInputElement>("#git-user-email")?.value.trim() || "";
  if (!email) {
    showToast("请先填写用于 SSH Key 注释的邮箱", true);
    return;
  }
  if (!window.confirm("将在当前用户 .ssh 目录生成 id_ed25519。已有同名密钥时会自动拒绝覆盖，确定继续吗？")) return;
  void runToolchainAction("git_generate_ssh", email);
});
document.querySelector("#test-github-ssh")?.addEventListener("click", () => void runToolchainAction("git_test_ssh"));
document.querySelector("#copy-public-key")?.addEventListener("click", () => {
  const publicKey = state.toolchains?.git.publicKey || "";
  if (!publicKey) {
    showToast("当前没有可复制的 SSH 公钥", true);
    return;
  }
  void copyText(publicKey);
});
document.querySelector("#set-npm-registry")?.addEventListener("click", () => {
  const value = document.querySelector<HTMLSelectElement>("#npm-registry")?.value || "official";
  void runToolchainAction("npm_registry", value);
});
document.querySelector("#set-pip-index")?.addEventListener("click", () => {
  const value = document.querySelector<HTMLSelectElement>("#pip-index")?.value || "official";
  void runToolchainAction("pip_index", value);
});
document.querySelector("#configure-env")?.addEventListener("click", () => {
  void runOperation(() => invoke<OperationResult>("configure_user_environment"), "正在配置用户环境变量");
});
document.querySelector("#check-env-health")?.addEventListener("click", async () => {
  showToast("正在检查环境配置");
  try {
    state.health = await invoke<EnvHealthCheck[]>("environment_health");
    renderHealth();
    showToast("环境检查完成");
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
document.querySelector("#cleanup-path")?.addEventListener("click", () => {
  void runOperation(() => invoke<OperationResult>("cleanup_path_entries"), "正在清理真实失效和重复 PATH");
});
document.querySelector("#restore-env")?.addEventListener("click", () => {
  void runOperation(() => invoke<OperationResult>("restore_user_environment"), "正在恢复用户环境变量");
});
document.querySelector("#save-profile")?.addEventListener("click", () => {
  const input = document.querySelector<HTMLInputElement>("#profile-name");
  const name = input?.value.trim() || "";
  void runOperation(
    () => invoke<OperationResult>("save_config_profile", { name }),
    "正在保存配置模板",
  );
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
document.querySelector("#self-uninstall")?.addEventListener("click", () => {
  const ok = window.confirm("这会启动 DevEnv Manager 的卸载程序并关闭当前程序。确定继续吗？");
  if (!ok) return;
  void runOperation(() => invoke<OperationResult>("self_uninstall"), "正在启动卸载程序");
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
  showToast("正在分析项目");
  try {
    const analysis = await invoke<ProjectAnalysis>("analyze_project", { path: input.value });
    renderProjectAnalysis(analysis);
    showToast("项目分析完成");
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
document.querySelector("#generate-vscode")?.addEventListener("click", () => {
  const input = document.querySelector<HTMLInputElement>("#project-path");
  if (!input) return;
  void runOperation(
    () => invoke<OperationResult>("generate_vscode_config", { projectPath: input.value }),
    "正在生成 VS Code 配置",
  );
});

document.querySelector("#port-search")?.addEventListener("input", (event) => {
  portState.query = (event.target as HTMLInputElement).value;
  renderPorts();
});

document.querySelector("#port-quick-filters")?.addEventListener("click", (event) => {
  const button = (event.target as HTMLElement).closest<HTMLButtonElement>("button[data-port-filter]");
  if (!button) return;
  portState.quickFilter = button.dataset.portFilter || "all";
  document.querySelectorAll(".filter-chip").forEach((item) => item.classList.toggle("active", item === button));
  renderPorts();
});

document.querySelectorAll<HTMLButtonElement>(".sort-head").forEach((button) => {
  button.addEventListener("click", () => {
    const key = button.dataset.sort as PortSortKey;
    if (portState.sortKey === key) {
      portState.sortDirection = portState.sortDirection === "asc" ? "desc" : "asc";
    } else {
      portState.sortKey = key;
      portState.sortDirection = ["localPort", "pid"].includes(key) ? "asc" : "asc";
    }
    renderPorts();
  });
});

document.addEventListener("click", (event) => {
  const button = (event.target as HTMLElement).closest<HTMLButtonElement>(
    "button[data-action], button[data-toolchain-action], button[data-python-tool]",
  );
  if (!button) return;
  const action = button.dataset.action;
  if (action === "doctor-fix") {
    const fix = button.dataset.fix || "";
    void runDoctorAction(fix).catch((error) => showToast(error instanceof Error ? error.message : String(error), true));
  }
  if (action === "copy-text") {
    void copyText(button.dataset.copy || "");
  }
  const toolchainAction = button.dataset.toolchainAction;
  if (toolchainAction) {
    void runToolchainAction(toolchainAction);
  }
  const pythonTool = button.dataset.pythonTool;
  if (pythonTool) {
    void runToolchainAction("python_install_tool", pythonTool);
  }
  if (action === "project-run") {
    const input = document.querySelector<HTMLInputElement>("#project-path");
    const output = document.querySelector<HTMLElement>("#project-output");
    const projectAction = button.dataset.projectAction || "";
    const command = state.project?.actions.find((item) => item.id === projectAction)?.command || "";
    if (!input) return;
    if (projectAction === "copy_commands") {
      void invoke<CommandRunResult>("run_project_action", { path: input.value, action: projectAction })
        .then((result) => copyText(result.output))
        .catch((error) => showToast(error instanceof Error ? error.message : String(error), true));
      return;
    }
    const longRunning = projectAction === "npm_dev" || projectAction === "npm_tauri_dev";
    const ok = longRunning || window.confirm(`将运行：${command}\n\n工作目录：${input.value}\n\n确定继续吗？`);
    if (!ok) return;
    showToast(longRunning ? "正在后台启动开发服务" : "正在运行项目命令");
    void invoke<CommandRunResult>("run_project_action", { path: input.value, action: projectAction })
      .then((result) => {
        if (output) {
          output.textContent = `退出码 ${result.returnCode} · ${result.elapsedMs} ms\n${result.output}`;
        }
        showToast(result.success ? "项目操作完成" : "项目操作失败", !result.success);
      })
      .catch((error) => showToast(error instanceof Error ? error.message : String(error), true));
  }
  if (action === "switch-jdk") {
    const version = button.dataset.version || "";
    const path = button.dataset.path || null;
    void runRuntimeOperation(
      () => invoke<OperationResult>("switch_runtime", { kind: "jdk", version, path }),
      `正在切换 JDK ${version}`,
      "JDK",
    );
  }
  if (action === "uninstall-jdk") {
    const version = button.dataset.version || "";
    const path = button.dataset.path || null;
    void runOperation(
      () => invoke<OperationResult>("uninstall_runtime", { kind: "jdk", version, path }),
      `正在卸载 JDK ${version}`,
    );
  }
  if (action === "switch-node") {
    const version = button.dataset.version || "";
    const path = button.dataset.path || null;
    void runRuntimeOperation(
      () => invoke<OperationResult>("switch_runtime", { kind: "node", version, path }),
      `正在切换 Node.js ${version}`,
      "Node.js",
    );
  }
  if (action === "uninstall-node") {
    const version = button.dataset.version || "";
    const path = button.dataset.path || null;
    void runOperation(
      () => invoke<OperationResult>("uninstall_runtime", { kind: "node", version, path }),
      `正在卸载 Node.js ${version}`,
    );
  }
  if (action === "switch-go") {
    const version = button.dataset.version || "";
    const path = button.dataset.path || null;
    void runRuntimeOperation(
      () => invoke<OperationResult>("switch_runtime", { kind: "go", version, path }),
      `正在切换 Go ${version}`,
      "Go",
    );
  }
  if (action === "uninstall-go") {
    const version = button.dataset.version || "";
    const path = button.dataset.path || null;
    void runOperation(
      () => invoke<OperationResult>("uninstall_runtime", { kind: "go", version, path }),
      `正在卸载 Go ${version}`,
    );
  }
  if (action === "switch-python") {
    const version = button.dataset.version || "";
    const path = button.dataset.path || null;
    void runRuntimeOperation(
      () => invoke<OperationResult>("switch_runtime", { kind: "python", version, path }),
      `正在切换 Python ${version}`,
      "Python",
    );
  }
  if (action === "uninstall-python") {
    const version = button.dataset.version || "";
    const path = button.dataset.path || null;
    void runOperation(
      () => invoke<OperationResult>("uninstall_runtime", { kind: "python", version, path }),
      `正在卸载 Python ${version}`,
    );
  }
  if (action === "switch-build-tool") {
    const kind = button.dataset.kind || "";
    const version = button.dataset.version || "";
    const path = button.dataset.path || null;
    void runRuntimeOperation(
      () => invoke<OperationResult>("switch_runtime", { kind, version, path }),
      `正在切换 ${kind} ${version}`,
      kind === "maven" ? "Maven" : "Gradle",
    );
  }
  if (action === "uninstall-build-tool") {
    const kind = button.dataset.kind || "";
    const version = button.dataset.version || "";
    const path = button.dataset.path || null;
    void runOperation(
      () => invoke<OperationResult>("uninstall_runtime", { kind, version, path }),
      `正在卸载 ${kind} ${version}`,
    );
  }
  if (action === "uninstall-external-runtime") {
    const kind = button.dataset.kind || "";
    const executable = button.dataset.executable || "";
    const ok = window.confirm(`将启动 Windows 卸载器来卸载 ${kind}。\n\n${executable}\n\n确定继续吗？`);
    if (!ok) return;
    void runOperation(
      () => invoke<OperationResult>("uninstall_external_runtime", { kind, executable }),
      `正在启动 ${kind} 的系统卸载程序`,
    );
  }
  if (action === "apply-profile") {
    const id = button.dataset.id || "";
    void runRuntimeOperation(
      () => invoke<OperationResult>("apply_config_profile", { id }),
      "正在应用配置模板",
      "PATH",
    );
  }
  if (action === "delete-profile") {
    const id = button.dataset.id || "";
    void runOperation(
      () => invoke<OperationResult>("delete_config_profile", { id }),
      "正在删除配置模板",
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
void refreshAll(false).then(() => {
  window.setTimeout(() => void refreshRuntimeAndPorts(true), 350);
});
