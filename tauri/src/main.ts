import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Activity,
  Boxes,
  Clipboard,
  Cpu,
  Database,
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
    autoCheckUpdate: boolean;
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

type JavaEnvironmentReport = {
  javaHome: string;
  javaHomeExpanded: string;
  pathJava: string;
  pathJavac: string;
  javaVersion: string;
  javacVersion: string;
  mavenRuntime: string;
  gradleRuntime: string;
  effectiveSource: string;
  consistent: boolean;
  warnings: string[];
  candidates: RuntimeInfo[];
};

type PortRecord = {
  protocol: string;
  localAddress: string;
  localPort: number;
  remoteAddress: string;
  state: string;
  pid: number;
  processName: string;
  processPath: string;
  commandLine: string;
  parentPid: number;
  parentProcessName: string;
  serviceNames: string[];
  commonUsage: string;
  explanation: string;
  risk: string;
};

type PortHistorySummary = {
  port: number;
  processName: string;
  observations: number;
  lastSeen: number;
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

type CommandSafetyAssessment = {
  allowed: boolean;
  risk: string;
  reason: string;
  requiresConfirmation: boolean;
  elevated: boolean;
  executable: string;
};

type AgentTraceReport = {
  generatedAt: string;
  items: Array<{
    source: string;
    path: string;
    evidence: string;
    confidence: string;
    recommendation: string;
  }>;
  privacyNotice: string;
  limitations: string[];
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
type ProjectPortConfig = {
  id: string;
  kind: string;
  file: string;
  currentPort: number;
  line: number;
  description: string;
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

type SystemPlatformReport = {
  docker: ToolState;
  dockerInfo: string;
  dockerDesktopPath: string;
  wsl: ToolState;
  wslStatus: string;
  wslDistributions: string[];
  wslItems: Array<{
    name: string;
    state: string;
    version: string;
    isDefault: boolean;
  }>;
};

type LocalServiceStatus = {
  id: string;
  name: string;
  port: number;
  occupied: boolean;
  pid: number;
  processName: string;
  processPath: string;
  serviceNames: string[];
  safeToStop: boolean;
  connectionCommand: string;
  installed: boolean;
  serviceName: string;
  serviceState: string;
  binaryPath: string;
};

type JdkDistribution = {
  id: string;
  name: string;
  recommended: boolean;
  supportsInstall: boolean;
  description: string;
};

type UpdateCheckResult = {
  currentVersion: string;
  latestVersion: string;
  updateAvailable: boolean;
  date: string;
  notes: string[];
  downloadUrl: string;
  sha256: string;
  checkedAt: string;
};

type CleanupArchitecture = {
  schemaVersion: number;
  status: string;
  categories: Array<{
    id: string;
    name: string;
    risk: string;
    scanOnly: boolean;
    cleanupEnabled: boolean;
    protectedPatterns: string[];
  }>;
  safetyRules: string[];
};

type DoctorRepairResult = {
  beforeScore: number;
  afterScore: number;
  applied: string[];
  remaining: string[];
  report: DoctorReport;
};

type ConfigProfileImportPreview = {
  source: string;
  exportedAt: string;
  profiles: Array<{
    name: string;
    current: Record<string, string | null>;
    missing: string[];
    willReplace: boolean;
  }>;
};

type ProfileRequirement = {
  kind: string;
  version: string;
  installed: boolean;
  autoInstallSupported: boolean;
};

type CleanupCandidate = {
  id: string;
  path: string;
  size: number;
  modifiedAt?: string;
  source: string;
  reason: string;
  risk: string;
  cleanable: boolean;
  selectedByDefault: boolean;
  skippedReason?: string;
};

type CleanupCategoryScan = {
  id: string;
  name: string;
  description: string;
  risk: string;
  scanOnly: boolean;
  cleanable: boolean;
  enabledByDefault: boolean;
  totalBytes: number;
  itemCount: number;
  items: CleanupCandidate[];
};

type CleanupScanReport = {
  generatedAt: string;
  totalBytes: number;
  totalItems: number;
  categories: CleanupCategoryScan[];
  warnings: string[];
};

type DiskVolumeInfo = {
  drive: string;
  totalBytes: number;
  freeBytes: number;
  usedBytes: number;
  usedPercent: number;
  fileSystem?: string;
  risk: string;
};

type MaintenanceOverview = {
  cDrive: DiskVolumeInfo;
  volumes: DiskVolumeInfo[];
  safeCleanEstimate: number;
  moveEstimate: number;
  devCacheEstimate: number;
  largeFileCount: number;
  startupCount: number;
  memorySummary?: {
    totalBytes: number;
    usedBytes: number;
    availableBytes: number;
    usedPercent: number;
  };
  riskLevel: string;
  summary: string;
  suggestions: string[];
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
        <button class="nav-item" data-view="runtimes">${icon(Terminal)}<span>版本管理</span></button>
        <button class="nav-item" data-view="environment">${icon(Route)}<span>环境</span></button>
        <button class="nav-item" data-view="project">${icon(FolderSearch)}<span>项目</span></button>
        <button class="nav-item" data-view="toolchains">${icon(PackageCheck)}<span>工具链</span></button>
        <button class="nav-item" data-view="platforms">${icon(Cpu)}<span>平台/镜像</span></button>
        <button class="nav-item" data-view="maintenance">${icon(Shield)}<span>C盘急救</span></button>
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
      <details id="view-guide" class="view-guide">
        <summary>${icon(FileText)}<span>这个页面怎么用？</span></summary>
        <p id="view-guide-text">先看系统快照和当前生效工具；需要深入排查时再进入环境医生。</p>
      </details>

      <section id="view-overview" class="view active">
        <div class="metrics">
          <article class="metric">
            <span>默认根目录</span>
            <strong id="metric-root">...</strong>
          </article>
          <article class="metric">
            <span>已发现工具</span>
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
        <section class="panel effective-environment-panel">
          <div class="panel-head">
            <div class="panel-title">${icon(Terminal)}<h2>当前实际生效环境</h2></div>
            <small>每 30 秒只读刷新</small>
          </div>
          <div id="effective-runtime-list" class="effective-runtime-list"><div class="empty">正在读取当前工具版本</div></div>
        </section>
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
              <button id="repair-doctor-safe">${icon(Hammer)}<span>安全修复</span></button>
              <button id="export-doctor">${icon(FileText)}<span>导出 Markdown</span></button>
              <button id="export-doctor-json">${icon(FileText)}<span>导出 JSON</span></button>
              <button id="copy-doctor-report">${icon(Clipboard)}<span>复制报告</span></button>
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
            <label class="toggle-row port-monitor-toggle" title="每 5 秒检查新出现的常用监听端口">
              <input id="port-monitor-enabled" type="checkbox" />
              <span>占用提醒</span>
            </label>
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
          <div class="grid two port-insights">
            <section id="port-detail" class="port-detail"><div class="empty">点击详情按钮查看端口解释</div></section>
            <section>
              <div class="panel-title compact-title">${icon(Activity)}<h2>最近 7 天</h2></div>
              <div id="port-history" class="runtime-list"><div class="empty">还没有端口历史</div></div>
            </section>
          </div>
        </section>
      </section>

      <section id="view-runtimes" class="view">
        <details class="panel discovery-panel">
          <summary><span>${icon(Terminal)}<strong>本机环境发现</strong><small>默认折叠，展开查看全部来源</small></span></summary>
          <div class="panel-head discovery-actions"><span></span><button id="discover-runtimes">${icon(RefreshCw)}<span>重新发现</span></button></div>
          <div id="runtime-list" class="runtime-list"></div>
        </details>
        <section class="panel runtime-manager">
          <div class="panel-head"><div class="panel-title">${icon(Download)}<h2>JDK 管理</h2></div><button id="inspect-java">${icon(Activity)}<span>检查当前 JDK</span></button></div>
          <div class="toolbar">
            <select id="jdk-distribution"></select>
            <select id="jdk-version">
              <option value="8">JDK 8</option>
              <option value="11">JDK 11</option>
              <option value="17">JDK 17</option>
              <option value="21" selected>JDK 21</option>
              <option value="25">JDK 25</option>
            </select>
            <button id="install-jdk">${icon(Download)}<span>安装</span></button>
          </div>
          <div id="java-environment" class="java-environment"><div class="empty">点击“检查当前 JDK”核对 JAVA_HOME、PATH、java、javac、Maven 和 Gradle</div></div>
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
            <div class="form-row profile-file-row">
              <input id="profile-file-path" placeholder="团队模板 JSON 文件路径" />
              <button id="preview-profiles">${icon(Search)}<span>预览</span></button>
              <button id="import-profiles" disabled>${icon(Download)}<span>确认导入</span></button>
              <button id="export-profiles">${icon(FileText)}<span>导出全部</span></button>
            </div>
            <div id="profile-import-preview"></div>
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

      <section id="view-maintenance" class="view maintenance-view">
        <section class="maintenance-hero">
          <div>
            <span class="phase-badge">Phase 1 · Scan only</span>
            <h2>C 盘急救大师</h2>
            <p>先看清空间去了哪里，再决定下一步。本阶段只读扫描，不删除、不移动、不修改任何文件。</p>
          </div>
          <div class="toolbar compact">
            <button id="inspect-maintenance" class="primary">${icon(Activity)}<span>开始体检</span></button>
            <button id="scan-maintenance">${icon(Search)}<span>只读扫描</span></button>
          </div>
        </section>
        <nav class="maintenance-tabs" aria-label="C 盘急救功能">
          <button class="active" data-maintenance-tab="overview">总览</button>
          <button data-maintenance-tab="cleanup">C盘专清</button>
          <button data-maintenance-tab="dev-cache">开发缓存</button>
          <button data-maintenance-tab="analysis">空间分析</button>
          <button data-maintenance-tab="move">空间搬家</button>
          <button data-maintenance-tab="expand">扩容检测</button>
          <button data-maintenance-tab="startup">启动项/进程</button>
          <button data-maintenance-tab="report">报告</button>
        </nav>

        <section class="maintenance-panel active" data-maintenance-panel="overview">
          <div id="maintenance-overview"><div class="empty">点击“开始体检”读取 C 盘与各分区容量</div></div>
        </section>
        <section class="maintenance-panel" data-maintenance-panel="cleanup">
          <div class="scan-only-banner">${icon(Shield)}<span>仅扫描：Windows Temp、回收站、错误报告和系统缓存都不会被清理。</span></div>
          <div id="maintenance-cleanup-categories"><div class="empty">点击“只读扫描”生成真实扫描结果</div></div>
        </section>
        <section class="maintenance-panel" data-maintenance-panel="dev-cache">
          <div class="scan-only-banner">${icon(Shield)}<span>仅扫描：npm、pnpm、Yarn、pip、uv、Poetry、Maven、Gradle、Cargo、Go 与 NuGet 缓存。</span></div>
          <div id="maintenance-dev-categories"><div class="empty">点击“只读扫描”统计开发缓存</div></div>
        </section>
        ${[
          ["analysis", "空间分析", "大文件明细与目录树将在后续阶段开放。"],
          ["move", "空间搬家", "后续阶段会提供可预览、可回滚的目录迁移。"],
          ["expand", "扩容检测", "后续阶段会检测分区布局与可扩容条件。"],
          ["startup", "启动项/进程", "Phase 1 仅在总览统计启动目录项目数量。"],
          ["report", "报告", "后续阶段会支持导出体检与执行记录。"],
        ].map(([id, title, text]) => `
          <section class="maintenance-panel" data-maintenance-panel="${id}">
            <div class="panel maintenance-placeholder"><h2>${title}</h2><p>${text}</p><span>Phase 1 暂未开放</span></div>
          </section>
        `).join("")}
      </section>

      <section id="view-toolbox" class="view">
        <div class="grid two">
          <section class="panel">
            <div class="panel-head">
              <div class="panel-title">${icon(Boxes)}<h2>Docker / WSL</h2></div>
              <button id="inspect-system-platforms">${icon(RefreshCw)}<span>检查</span></button>
            </div>
            <div id="system-platform-result" class="platform-content"><div class="empty">尚未检查 Docker 与 WSL</div></div>
            <div class="toolbar system-platform-actions">
              <button id="open-docker-desktop">${icon(Play)}<span>启动 Docker Desktop</span></button>
              <button data-action="system-platform" data-platform-action="docker_install">${icon(Download)}<span>安装 Docker</span></button>
              <button data-action="system-platform" data-platform-action="docker_update">${icon(RefreshCw)}<span>升级 Docker</span></button>
              <button data-action="system-platform" data-platform-action="docker_shutdown">${icon(Trash2)}<span>退出 Docker</span></button>
              <button data-action="system-platform" data-platform-action="wsl_install">${icon(Download)}<span>安装 WSL</span></button>
              <button data-action="system-platform" data-platform-action="wsl_update">${icon(RefreshCw)}<span>更新 WSL</span></button>
            </div>
            <div class="form-row wsl-install-row">
              <input id="wsl-distro-name" value="Ubuntu" placeholder="WSL 在线发行版名称" />
              <button data-action="system-platform" data-platform-action="wsl_install_distro">${icon(Download)}<span>安装发行版</span></button>
            </div>
            <div class="small-note">Windows 主机与 WSL 是两套独立环境。本项目当前管理发行版状态；WSL 内的 SDK 优先交给 mise/asdf/sdkman/pyenv/nvm/rustup 等成熟工具。</div>
          </section>
          <section class="panel">
            <div class="panel-head">
              <div class="panel-title">${icon(Database)}<h2>数据库与本地服务</h2></div>
              <button id="inspect-local-services">${icon(RefreshCw)}<span>检查</span></button>
            </div>
            <div id="local-service-result" class="runtime-list"><div class="empty">尚未检查常见开发服务</div></div>
            <pre id="local-service-logs" class="command-output compact-output service-log-output">选择已安装服务查看最近日志</pre>
          </section>
        </div>
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
          <div class="advanced-warning">${icon(Shield)}<span><strong>高级功能 · 安全模式</strong> 仅允许常见开发工具；系统 Shell、磁盘、注册表、权限和破坏性 Git 命令会被后端拒绝。不要粘贴不理解的 AI/网页命令。</span></div>
          <div class="form-row command-row">
            <input id="command-input" value="node --version" />
            <input id="command-cwd" placeholder="工作目录，可留空" />
            <button id="run-command">${icon(Play)}<span>运行</span></button>
          </div>
          <pre id="command-output" class="command-output"></pre>
        </section>
        <section class="panel runtime-manager">
          <div class="panel-head">
            <div class="panel-title">${icon(Search)}<h2>AI Agent / CLI 痕迹</h2></div>
            <button id="inspect-agent-traces">${icon(Shield)}<span>只读分析</span></button>
          </div>
          <div class="small-note">只有主动点击后才读取本地路径和文件名；不读取会话正文、shell history、token 或密钥，不上传任何数据。</div>
          <div id="agent-trace-result"><div class="empty">尚未分析 Agent / CLI 痕迹</div></div>
        </section>
        <section class="panel runtime-manager">
          <div class="panel-head">
            <div class="panel-title">${icon(RefreshCw)}<h2>版本更新</h2></div>
            <div class="toolbar compact">
              <label class="toggle-row" title="程序启动后在后台检查 GitHub 更新清单">
                <input id="auto-check-updates" type="checkbox" />
                <span>启动时检查（每天最多一次，仅取版本号，不上传遥测）</span>
              </label>
              <button id="check-updates">${icon(RefreshCw)}<span>检查更新</span></button>
            </div>
          </div>
          <div id="update-result"><div class="empty">尚未检查新版本</div></div>
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
          <div class="project-port-section">
            <div class="panel-head"><div class="panel-title">${icon(Network)}<h2>项目端口配置</h2></div><button id="inspect-project-ports">${icon(Search)}<span>分析端口</span></button></div>
            <div id="project-port-configs" class="runtime-list"><div class="empty">分析项目后显示可安全修改的端口配置</div></div>
          </div>
      </section>
    </section>
  </main>
`;

const state = {
  snapshot: null as AppSnapshot | null,
  env: null as EnvSnapshot | null,
  config: null as ConfigView | null,
  runtimes: [] as RuntimeInfo[],
  javaEnvironment: null as JavaEnvironmentReport | null,
  ports: [] as PortRecord[],
  portHistory: [] as PortHistorySummary[],
  selectedPort: null as PortRecord | null,
  network: null as NetworkDiagnostics | null,
  cache: [] as CacheEntry[],
  health: [] as EnvHealthCheck[],
  profiles: [] as ConfigProfile[],
  profileImportPreview: null as ConfigProfileImportPreview | null,
  doctor: null as DoctorReport | null,
  python: null as PythonAnalysis | null,
  project: null as ProjectAnalysis | null,
  toolchains: null as ToolchainReport | null,
  platforms: null as PlatformReport | null,
  projectPorts: [] as ProjectPortConfig[],
  systemPlatforms: null as SystemPlatformReport | null,
  localServices: [] as LocalServiceStatus[],
  jdkDistributions: [] as JdkDistribution[],
  update: null as UpdateCheckResult | null,
  updateError: "",
  updateDownloaded: false,
  agentTraces: null as AgentTraceReport | null,
  cleanupArchitecture: null as CleanupArchitecture | null,
  cleanupReport: null as CleanupScanReport | null,
  maintenanceOverview: null as MaintenanceOverview | null,
};

const portState = {
  sortKey: "localPort" as PortSortKey,
  sortDirection: "asc" as SortDirection,
  query: "",
  quickFilter: "all",
};
let portMonitorTimer: number | null = null;
let knownListeningPorts = new Set<string>();

async function pollPortMonitor(initial = false) {
  try {
    const records = await invoke<PortRecord[]>("scan_ports");
    const listening = records.filter((item) => item.state.toLowerCase() === "listening");
    const current = new Set(listening.map((item) => `${item.protocol}:${item.localPort}:${item.pid}`));
    if (!initial) {
      const appeared = listening.filter((item) => {
        const key = `${item.protocol}:${item.localPort}:${item.pid}`;
        return !knownListeningPorts.has(key) && (portHint(item) || item.risk !== "普通");
      });
      if (appeared.length) {
        const summary = appeared
          .slice(0, 3)
          .map((item) => `${item.localPort} ${item.processName}`)
          .join("、");
        showToast(`发现新的常用端口占用：${summary}`);
      }
    }
    knownListeningPorts = current;
    state.ports = records;
    renderPorts();
  } catch {
    // 监控失败保持静默，手动扫描仍会显示错误。
  }
}


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
  renderEffectiveRuntimes();
  const element = document.querySelector<HTMLElement>("#runtime-list");
  if (!element) return;
  element.innerHTML = state.runtimes.length
    ? state.runtimes
        .map(
          (runtime) => `
            <article class="runtime">
              <div><strong>${escapeHtml(runtime.kind)}</strong><span>${escapeHtml(runtime.version)}</span></div>
              <small>${escapeHtml(runtime.source)} · ${escapeHtml(runtime.executable)}</small>
              ${canUninstallExternal(runtime) ? `<div class="row-actions"><button data-action="uninstall-external-runtime" data-kind="${escapeHtml(runtime.kind)}" data-source="${escapeHtml(runtime.source)}" data-executable="${escapeHtml(runtime.executable)}">${icon(Trash2)}<span>${runtime.source === "Scoop" || runtime.source === "Chocolatey" ? "用包管理器卸载" : "系统卸载"}</span></button></div>` : ""}
            </article>
          `,
        )
        .join("")
    : `<div class="empty">还没有发现开发工具</div>`;
  renderManagedJdks();
  renderManagedNodes();
  renderManagedPythons();
  renderManagedBuildTools();
  renderManagedGos();
}

function renderEffectiveRuntimes() {
  const element = document.querySelector<HTMLElement>("#effective-runtime-list");
  if (!element) return;
  const preferredKinds = ["Java", "Python", "Node.js", "Maven", "Gradle", "Go"];
  const effective = preferredKinds
    .map((kind) => state.runtimes.find((runtime) => runtime.kind === kind))
    .filter((runtime): runtime is RuntimeInfo => Boolean(runtime));
  element.innerHTML = effective.length
    ? effective.map((runtime) => `
        <article>
          <span>${escapeHtml(runtime.kind)}</span>
          <strong>${escapeHtml(runtime.version.split("\n")[0] || "未知版本")}</strong>
          <small>${escapeHtml(runtime.source)} · ${escapeHtml(runtime.executable)}</small>
        </article>
      `).join("")
    : `<div class="empty">尚未发现当前可执行工具</div>`;
}

function renderJavaEnvironment() {
  const element = document.querySelector<HTMLElement>("#java-environment");
  const report = state.javaEnvironment;
  if (!element || !report) return;
  element.innerHTML = `
    <div class="java-health ${report.consistent ? "ok" : "warn"}">
      <strong>${report.consistent ? "JDK 生效链一致" : "JDK 生效链存在冲突"}</strong>
      <span>实际来源：${escapeHtml(report.effectiveSource)}</span>
    </div>
    <div class="kv-list">
      <div><span>JAVA_HOME</span><strong>${escapeHtml(report.javaHome || "未设置")}</strong></div>
      <div><span>PATH java</span><strong>${escapeHtml(report.pathJava || "未发现")}</strong></div>
      <div><span>PATH javac</span><strong>${escapeHtml(report.pathJavac || "未发现")}</strong></div>
      <div><span>java -version</span><strong>${escapeHtml(report.javaVersion || "未读取")}</strong></div>
      <div><span>javac -version</span><strong>${escapeHtml(report.javacVersion || "未读取")}</strong></div>
      <div><span>Maven 使用环境</span><strong>${escapeHtml(report.mavenRuntime || "未安装/未读取")}</strong></div>
      <div><span>Gradle 使用 JVM</span><strong>${escapeHtml(report.gradleRuntime || "未安装/未读取")}</strong></div>
    </div>
    ${report.warnings.length ? `<ul class="warning-text">${report.warnings.map((warning) => `<li>${escapeHtml(warning)}</li>`).join("")}</ul>` : ""}
  `;
}

async function inspectJava(showProgress = true) {
  if (showProgress) showToast("正在核对 JAVA_HOME、PATH、java、javac、Maven 与 Gradle");
  try {
    state.javaEnvironment = await invoke<JavaEnvironmentReport>("inspect_java_environment");
    renderJavaEnvironment();
    if (showProgress) showToast(state.javaEnvironment.consistent ? "当前 JDK 生效链一致" : "发现 JDK 生效冲突", !state.javaEnvironment.consistent);
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
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

function renderJdkDistributions() {
  const select = document.querySelector<HTMLSelectElement>("#jdk-distribution");
  if (!select) return;
  select.innerHTML = state.jdkDistributions
    .map(
      (item) =>
        `<option value="${escapeHtml(item.id)}" ${item.supportsInstall ? "" : "disabled"}>${escapeHtml(item.name)}${item.recommended ? " · 推荐" : item.supportsInstall ? "" : " · 仅检测"}</option>`,
    )
    .join("");
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
          <td><div class="table-actions"><button class="icon-action" data-action="port-details" data-pid="${record.pid}" data-port="${record.localPort}" title="查看详情">${icon(Search)}</button><button class="icon-action" data-action="kill-port" data-pid="${record.pid}" title="结束进程">${icon(Trash2)}</button></div></td>
        </tr>
      `;
      },
    )
    .join("");
  updateSortHeaders();
  renderPortDetails();
  renderPortHistory();
}

function renderPortDetails() {
  const element = document.querySelector<HTMLElement>("#port-detail");
  if (!element) return;
  const record = state.selectedPort;
  if (!record) {
    element.innerHTML = `<div class="empty">点击详情按钮查看端口解释</div>`;
    return;
  }
  element.innerHTML = `
    <div class="panel-title compact-title">${icon(Network)}<h2>${record.localPort} · ${escapeHtml(record.commonUsage)}</h2></div>
    <p class="port-explanation">${escapeHtml(record.explanation)}</p>
    <div class="kv-list port-detail-kv">
      <div><span>进程</span><strong>${escapeHtml(record.processName)} / PID ${record.pid}</strong></div>
      <div><span>进程路径</span><strong>${escapeHtml(record.processPath || "未读取")}</strong></div>
      <div><span>启动命令</span><strong>${escapeHtml(record.commandLine || "未读取")}</strong></div>
      <div><span>父进程</span><strong>${escapeHtml(record.parentProcessName || "未读取")}${record.parentPid ? ` / PID ${record.parentPid}` : ""}</strong></div>
      <div><span>Windows 服务</span><strong>${escapeHtml(record.serviceNames.join("、") || "未识别")}</strong></div>
      <div><span>风险</span><strong>${escapeHtml(record.risk)}</strong></div>
    </div>
    <div class="toolbar compact port-detail-actions">
      <button data-action="copy-text" data-copy="${escapeHtml(record.processPath)}">${icon(Clipboard)}<span>复制路径</span></button>
      ${record.processPath ? `<button data-action="open-process-location" data-pid="${record.pid}">${icon(FolderSearch)}<span>打开位置</span></button>` : ""}
      <button data-action="copy-text" data-copy="taskkill /PID ${record.pid} /T">${icon(Clipboard)}<span>复制命令</span></button>
      <button class="danger-button" data-action="kill-port" data-pid="${record.pid}">${icon(Trash2)}<span>安全结束</span></button>
    </div>
  `;
}

function renderPortHistory() {
  const element = document.querySelector<HTMLElement>("#port-history");
  if (!element) return;
  element.innerHTML = state.portHistory.length
    ? state.portHistory
        .slice(0, 12)
        .map(
          (item) => `
            <article class="runtime">
              <div><strong>${item.port} · ${escapeHtml(item.processName)}</strong><span>${item.observations} 次</span></div>
              <small>最近记录：${new Date(item.lastSeen * 1000).toLocaleString("zh-CN")}</small>
            </article>
          `,
        )
        .join("")
    : `<div class="empty">还没有端口历史</div>`;
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
                <button data-action="install-apply-profile" data-id="${escapeHtml(profile.id)}">${icon(Download)}<span>补齐并应用</span></button>
                <button data-action="delete-profile" data-id="${escapeHtml(profile.id)}">${icon(Trash2)}<span>删除</span></button>
              </div>
            </article>
          `;
        })
        .join("")
    : `<div class="empty">还没有保存配置模板</div>`;
}

function renderProfileImportPreview() {
  const element = document.querySelector<HTMLElement>("#profile-import-preview");
  const importButton = document.querySelector<HTMLButtonElement>("#import-profiles");
  if (!element || !importButton) return;
  const preview = state.profileImportPreview;
  importButton.disabled = !preview;
  element.innerHTML = preview
    ? `<div class="profile-preview">
        <div class="project-summary"><strong>${preview.profiles.length} 个模板</strong><span>${escapeHtml(preview.source)}</span></div>
        <div class="runtime-list">
          ${preview.profiles.map((profile) => `
            <article class="runtime">
              <div><strong>${escapeHtml(profile.name)}</strong><span>${profile.willReplace ? "将覆盖同名模板" : "新增"}</span></div>
              <small>${Object.entries(profile.current).filter(([, value]) => value).map(([kind, value]) => `${kind} ${value}`).join(" · ") || "仅环境变量"}</small>
              <small>${profile.missing.length ? `缺失：${escapeHtml(profile.missing.join("、"))}` : "所需运行时均已安装"}</small>
            </article>
          `).join("")}
        </div>
      </div>`
    : "";
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
  score.innerHTML = `<strong>${report.score}</strong><span>${escapeHtml(report.summary)}</span>`;
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
  const categories = report.checks.reduce<Record<string, DoctorReport["checks"]>>((groups, item) => {
    (groups[item.category] ||= []).push(item);
    return groups;
  }, {});
  checks.innerHTML = Object.entries(categories)
    .map(
      ([category, items]) => `
        <section class="doctor-category">
          <div class="doctor-category-title"><h3>${escapeHtml(category)}</h3><span>${items.length} 项</span></div>
          ${items
            .map(
              (item) => `
                <article class="doctor-check ${escapeHtml(item.severity)}">
                  <div class="doctor-check-main">
                    <div class="doctor-check-heading">
                      <strong>${escapeHtml(item.title)}</strong>
                      <span>${escapeHtml(item.status)}</span>
                    </div>
                    <small>${escapeHtml(item.detail || "无详情")}</small>
                  </div>
                  ${item.fixAction ? `<button data-action="doctor-fix" data-fix="${escapeHtml(item.fixAction)}">${doctorActionLabel(item.fixAction)}</button>` : ""}
                </article>
              `,
            )
            .join("")}
        </section>
      `,
    )
    .join("");
}

function doctorActionLabel(action: string) {
  const labels: Record<string, string> = {
    cleanup_path: "清理 PATH",
    configure_env: "配置环境",
    discover_runtimes: "刷新版本列表",
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
      record.processPath,
      record.commandLine,
      record.parentProcessName,
      record.serviceNames.join(" "),
      record.commonUsage,
      record.explanation,
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
  if (record.commonUsage && record.commonUsage !== "未识别的本地服务") exact.push(record.commonUsage);
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
            .join("") || `<div class="empty">没有特殊版本要求</div>`}
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

function renderProjectPortConfigs() {
  const element = document.querySelector<HTMLElement>("#project-port-configs");
  if (!element) return;
  element.innerHTML = state.projectPorts.length
    ? state.projectPorts.map((config) => `
        <article class="runtime project-port-item">
          <div><strong>${escapeHtml(config.description)}</strong><span>当前 ${config.currentPort}</span></div>
          <small>${escapeHtml(config.file)}${config.line ? ` · 第 ${config.line} 行` : " · 将创建配置"}</small>
          <div class="row-actions port-config-actions">
            <input type="number" min="1024" max="65535" value="${config.currentPort + 1}" data-port-config-input="${escapeHtml(config.id)}" aria-label="新端口" />
            <button data-action="update-project-port" data-config-id="${escapeHtml(config.id)}">${icon(RefreshCw)}<span>备份并修改</span></button>
          </div>
        </article>
      `).join("")
    : `<div class="empty">没有发现可修改的 Spring Boot、Tomcat、Vite 或 .env 端口配置</div>`;
}

async function inspectProjectPorts(showProgress = true) {
  const path = document.querySelector<HTMLInputElement>("#project-path")?.value.trim() || "";
  if (!path) return;
  if (showProgress) showToast("正在分析项目端口配置");
  try {
    state.projectPorts = await invoke<ProjectPortConfig[]>("inspect_project_port_configs", { path });
    renderProjectPortConfigs();
    if (showProgress) showToast(`发现 ${state.projectPorts.length} 个端口配置`);
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
}

async function refreshBase() {
  const [snapshot, config, envSnapshot, profiles, jdkDistributions, cleanupArchitecture] = await Promise.all([
    invoke<AppSnapshot>("app_snapshot"),
    invoke<ConfigView>("load_config"),
    invoke<EnvSnapshot>("env_snapshot"),
    invoke<ConfigProfile[]>("list_config_profiles"),
    invoke<JdkDistribution[]>("jdk_distributions"),
    invoke<CleanupArchitecture>("storage_cleanup_architecture"),
  ]);

  state.snapshot = snapshot;
  state.config = config;
  state.env = envSnapshot;
  state.profiles = profiles;
  state.jdkDistributions = jdkDistributions;
  state.cleanupArchitecture = cleanupArchitecture;
  renderSnapshot();
  renderEnv();
  renderHealth();
  renderProfiles();
  renderProfileImportPreview();
  renderDoctor();
  renderPythonAnalysis();
  renderRuntimes();
  renderJdkDistributions();
  renderJavaEnvironment();
  renderAgentTraces();
  renderUpdate();
  renderMaintenanceOverview();
  renderMaintenanceScan();
  renderPorts();
  const autoCheckUpdates = document.querySelector<HTMLInputElement>("#auto-check-updates");
  if (autoCheckUpdates) autoCheckUpdates.checked = config.settings.autoCheckUpdate;
}

async function refreshRuntimeAndPorts(silent = false) {
  try {
    const [runtimes, ports] = await Promise.all([
      invoke<RuntimeInfo[]>("discover_runtimes"),
      invoke<PortRecord[]>("scan_ports"),
    ]);
    state.runtimes = runtimes;
    state.ports = ports;
    state.portHistory = await invoke<PortHistorySummary[]>("port_history");
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
    if (focus === "JDK") await inspectJava(false);
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

async function terminatePortProcess(pid: number) {
  const record = state.ports.find((item) => item.pid === pid);
  const label = record ? `${record.processName} / PID ${pid}` : `PID ${pid}`;
  if (!window.confirm(`将结束 ${label} 及其子进程。确定继续吗？`)) return;
  try {
    let result = await invoke<KillResult>("kill_process", { pid, force: false, allowCaution: false });
    if (result.needsForce) {
      const force = window.confirm(`${result.message}\n\n是否改为强制结束？`);
      if (!force) {
        showToast("已取消强制结束");
        return;
      }
      result = await invoke<KillResult>("kill_process", { pid, force: true, allowCaution: false });
    }
    showToast(result.message, !result.success);
    state.ports = await invoke<PortRecord[]>("scan_ports");
    state.portHistory = await invoke<PortHistorySummary[]>("port_history");
    state.selectedPort = null;
    renderPorts();
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
    showToast("版本列表刷新完成");
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

function renderSystemPlatforms() {
  const element = document.querySelector<HTMLElement>("#system-platform-result");
  const report = state.systemPlatforms;
  if (!element || !report) return;
  element.innerHTML = `
    <div class="tool-state-grid">${renderToolStates([report.docker, report.wsl])}</div>
    <div class="kv-list toolchain-kv">
      <div><span>Docker Engine</span><strong>${escapeHtml(report.dockerInfo)}</strong></div>
      <div><span>Docker Desktop</span><strong>${escapeHtml(report.dockerDesktopPath || "未发现")}</strong></div>
      <div><span>WSL 状态</span><strong>${escapeHtml(report.wslStatus || "未读取")}</strong></div>
    </div>
    <div class="runtime-list wsl-list">
      ${report.wslItems.map((item) => `
        <article class="runtime">
          <div><strong>${escapeHtml(item.name)}</strong><span>${item.isDefault ? "默认 · " : ""}${escapeHtml(item.state)} · WSL ${escapeHtml(item.version)}</span></div>
          <div class="row-actions">
            <button data-action="system-platform" data-platform-action="wsl_start" data-platform-value="${escapeHtml(item.name)}">${icon(Play)}<span>启动</span></button>
            <button data-action="system-platform" data-platform-action="wsl_set_default" data-platform-value="${escapeHtml(item.name)}">${icon(RefreshCw)}<span>设为默认</span></button>
            <button data-action="system-platform" data-platform-action="wsl_terminate" data-platform-value="${escapeHtml(item.name)}">${icon(Trash2)}<span>终止</span></button>
          </div>
        </article>
      `).join("") || `<div class="empty">没有发现 WSL 发行版</div>`}
    </div>
  `;
}

function renderLocalServices() {
  const element = document.querySelector<HTMLElement>("#local-service-result");
  if (!element) return;
  element.innerHTML = state.localServices.length
    ? state.localServices
        .map(
          (service) => `
            <article class="runtime ${service.occupied ? "warn" : service.installed ? "ok" : ""}">
              <div>
                <strong>${escapeHtml(service.name)} · ${service.port}</strong>
                <span>${service.occupied ? "运行中" : service.installed ? `已安装 · ${escapeHtml(service.serviceState)}` : "未安装"}</span>
              </div>
              <small>${service.occupied ? `${escapeHtml(service.processName)} / PID ${service.pid}` : service.binaryPath ? escapeHtml(service.binaryPath) : "端口空闲"}</small>
              <div class="row-actions">
                <button data-action="copy-text" data-copy="${escapeHtml(service.connectionCommand)}">${icon(Clipboard)}<span>复制连接命令</span></button>
                ${service.installed && service.serviceName ? `
                  ${service.serviceState.toLowerCase() !== "running" ? `<button data-action="local-service-manage" data-service-action="start" data-service="${escapeHtml(service.serviceName)}">${icon(Play)}<span>启动</span></button>` : ""}
                  <button data-action="local-service-manage" data-service-action="restart" data-service="${escapeHtml(service.serviceName)}">${icon(RefreshCw)}<span>重启</span></button>
                  ${service.serviceState.toLowerCase() === "running" ? `<button class="danger-button" data-action="local-service-manage" data-service-action="stop" data-service="${escapeHtml(service.serviceName)}">${icon(Trash2)}<span>停止</span></button>` : ""}
                  <button data-action="local-service-logs" data-service="${escapeHtml(service.serviceName)}">${icon(FileText)}<span>日志</span></button>
                  <button data-action="local-service-directory" data-service="${escapeHtml(service.serviceName)}">${icon(FolderSearch)}<span>目录</span></button>
                ` : ""}
              </div>
            </article>
          `,
        )
        .join("")
    : `<div class="empty">尚未检查常见开发服务</div>`;
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

function renderUpdate() {
  const element = document.querySelector<HTMLElement>("#update-result");
  const update = state.update;
  if (!element) return;
  if (!update) {
    element.innerHTML = state.updateError
      ? `<div class="empty warning-text">最近检查失败：${escapeHtml(state.updateError)}</div>`
      : `<div class="empty">尚未检查新版本</div>`;
    return;
  }
  element.innerHTML = `
    <div class="project-summary">
      <strong>当前 ${escapeHtml(update.currentVersion)} · 最新 ${escapeHtml(update.latestVersion)}</strong>
      <span>${update.updateAvailable ? "发现新版本" : "当前已是最新版本"} · 发布 ${escapeHtml(update.date)} · 检查 ${escapeHtml(update.checkedAt)}</span>
    </div>
    ${update.notes.length ? `<ul>${update.notes.map((note) => `<li>${escapeHtml(note)}</li>`).join("")}</ul>` : ""}
    <div class="toolbar compact">
      ${update.updateAvailable ? `<button data-action="download-update">${icon(Download)}<span>${state.updateDownloaded ? "重新校验下载" : "下载更新"}</span></button>` : ""}
      ${update.updateAvailable && state.updateDownloaded ? `<button class="primary" data-action="install-update">${icon(Play)}<span>安装并重启</span></button>` : ""}
      <button data-action="copy-text" data-copy="${escapeHtml(update.downloadUrl)}">${icon(Clipboard)}<span>复制 Releases 地址</span></button>
    </div>
  `;
}

function riskText(risk: string) {
  return ({ critical: "严重", high: "高", medium: "中", low: "低", unknown: "未知" } as Record<string, string>)[risk] || risk;
}

function renderAgentTraces() {
  const element = document.querySelector<HTMLElement>("#agent-trace-result");
  const report = state.agentTraces;
  if (!element || !report) return;
  element.innerHTML = `
    <div class="privacy-notice">${icon(Shield)}<span>${escapeHtml(report.privacyNotice)}</span></div>
    <div class="runtime-list agent-trace-list">
      ${report.items.map((item) => `
        <article class="runtime">
          <div><strong>${escapeHtml(item.source)}</strong><span>置信度：${escapeHtml(item.confidence)}</span></div>
          <small>${escapeHtml(item.path)}</small>
          <small>${escapeHtml(item.evidence)}</small>
          <small>${escapeHtml(item.recommendation)}</small>
        </article>
      `).join("") || `<div class="empty">没有发现可验证的 Agent / CLI 安装痕迹</div>`}
    </div>
    <ul>${report.limitations.map((item) => `<li>${escapeHtml(item)}</li>`).join("")}</ul>
  `;
}

function renderMaintenanceOverview() {
  const element = document.querySelector<HTMLElement>("#maintenance-overview");
  const overview = state.maintenanceOverview;
  if (!element || !overview) return;
  const cDrive = overview.cDrive;
  element.innerHTML = `
    <div class="maintenance-metrics">
      <article class="maintenance-metric risk-${escapeHtml(overview.riskLevel)}">
        <span>C 盘剩余</span><strong>${formatBytes(cDrive.freeBytes || 0)}</strong>
        <small>${cDrive.totalBytes ? `已用 ${cDrive.usedPercent.toFixed(1)}%` : "未识别 C 盘"} · ${riskText(overview.riskLevel)}风险</small>
      </article>
      <article class="maintenance-metric"><span>可清理空间估算</span><strong>${formatBytes(overview.safeCleanEstimate)}</strong><small>仅估算，本阶段不执行</small></article>
      <article class="maintenance-metric"><span>开发缓存</span><strong>${formatBytes(overview.devCacheEstimate)}</strong><small>包管理器与构建缓存</small></article>
      <article class="maintenance-metric protected"><span>个人目录</span><strong>默认排除</strong><small>桌面、下载、文档、图片、视频、音乐均不进入扫描</small></article>
    </div>
    <div class="grid two maintenance-grid">
      <section class="panel">
        <div class="panel-title">${icon(Database)}<h2>磁盘容量</h2></div>
        <div class="volume-list">
          ${overview.volumes.map((volume) => `
            <article class="volume-row">
              <div><strong>${escapeHtml(volume.drive)}</strong><span class="risk-chip risk-${escapeHtml(volume.risk)}">${riskText(volume.risk)}风险</span></div>
              <div class="volume-track"><span style="width:${Math.min(100, volume.usedPercent).toFixed(1)}%"></span></div>
              <small>已用 ${formatBytes(volume.usedBytes)} / ${formatBytes(volume.totalBytes)} · 剩余 ${formatBytes(volume.freeBytes)}${volume.fileSystem ? ` · ${escapeHtml(volume.fileSystem)}` : ""}</small>
            </article>
          `).join("") || `<div class="empty">没有读取到磁盘卷</div>`}
        </div>
      </section>
      <section class="panel maintenance-advice">
        <div class="panel-title">${icon(Shield)}<h2>体检结论</h2></div>
        <p>${escapeHtml(overview.summary)}</p>
        <div class="maintenance-facts">
          <span>大目录 ${overview.largeFileCount} 个</span><span>启动目录项目 ${overview.startupCount} 个</span>
          ${overview.memorySummary ? `<span>内存已用 ${overview.memorySummary.usedPercent.toFixed(1)}%</span>` : ""}
        </div>
        <ul>${overview.suggestions.map((suggestion) => `<li>${escapeHtml(suggestion)}</li>`).join("")}</ul>
      </section>
    </div>
  `;
}

function renderScanCategories(target: string, categoryIds: string[]) {
  const element = document.querySelector<HTMLElement>(target);
  if (!element) return;
  const report = state.cleanupReport;
  if (!report) return;
  const categories = report.categories.filter((category) => categoryIds.includes(category.id));
  element.innerHTML = `
    <div class="scan-summary"><strong>${formatBytes(categories.reduce((sum, item) => sum + item.totalBytes, 0))}</strong><span>${categories.reduce((sum, item) => sum + item.itemCount, 0)} 个只读统计项</span></div>
    <div class="maintenance-category-list">
      ${categories.map((category) => `
        <details class="maintenance-category" ${category.totalBytes ? "open" : ""}>
          <summary>
            <span><strong>${escapeHtml(category.name)}</strong><small>${escapeHtml(category.description)}</small></span>
            <span><b>${formatBytes(category.totalBytes)}</b><i class="risk-chip risk-${escapeHtml(category.risk)}">${riskText(category.risk)}风险</i></span>
          </summary>
          <div class="maintenance-items">
            ${category.items.map((item) => `
              <article class="maintenance-item">
                <div><strong>${escapeHtml(item.source)}</strong><span>${formatBytes(item.size)}</span></div>
                <small>${escapeHtml(item.path)}</small>
                <small>${escapeHtml(item.skippedReason || item.reason)} · ${item.cleanable ? "未来可评估清理" : "受保护 / 仅统计"}</small>
              </article>
            `).join("") || `<div class="empty">目录不存在或占用为 0</div>`}
          </div>
        </details>
      `).join("")}
    </div>
    <ul class="scan-warnings">${report.warnings.map((warning) => `<li>${escapeHtml(warning)}</li>`).join("")}</ul>
  `;
}

function renderMaintenanceScan() {
  renderScanCategories("#maintenance-cleanup-categories", ["windows-temp", "system-caches", "recycle-bin", "devenv-manager", "wps-cache"]);
  renderScanCategories("#maintenance-dev-categories", ["developer-caches"]);
}

async function inspectMaintenance() {
  showToast("正在进行 C 盘只读体检，较大的缓存目录可能需要一些时间");
  try {
    state.maintenanceOverview = await invoke<MaintenanceOverview>("inspect_maintenance_overview");
    renderMaintenanceOverview();
    showToast(`体检完成：C 盘${riskText(state.maintenanceOverview.riskLevel)}风险`);
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
}

async function scanMaintenance() {
  showToast("正在执行只读扫描，不会删除任何文件");
  try {
    state.cleanupReport = await invoke<CleanupScanReport>("scan_cleanup_targets");
    renderMaintenanceScan();
    showToast(`扫描完成：${state.cleanupReport.totalItems} 项，共 ${formatBytes(state.cleanupReport.totalBytes)}`);
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
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
  const guides: Record<string, string> = {
    overview: "先确认当前实际生效的工具版本与路径；这里只读刷新，不会修改环境。",
    doctor: "先运行一键诊断，再逐条查看证据。安全修复只处理用户级 PATH 与受管环境变量。",
    ports: "搜索端口、进程或框架名称；结束进程前务必确认它不是系统或仍在使用的服务。",
    runtimes: "本机发现默认折叠。JDK 切换后请用“检查当前 JDK”核对 JAVA_HOME、PATH、java 和 javac。",
    environment: "配置操作只写当前用户环境变量，并保留快照；新终端或 IDE 才会继承修改。",
    project: "选择项目根目录后只读分析配置；运行按钮只接受后端生成的固定 action id。",
    toolchains: "优先检测并调用 Git、npm、pnpm、uv 等成熟工具，不替代它们。",
    platforms: "用于诊断 Go、Rust、.NET 与镜像配置；写配置前会备份或明确确认。",
    maintenance: "默认不进入任何个人目录；当前版本只扫描并预览，不删除文件。",
    toolbox: "命令面板是高级功能且启用白名单；不要粘贴不理解的 AI 或网页命令。",
  };
  const guide = document.querySelector<HTMLElement>("#view-guide-text");
  if (guide) guide.textContent = guides[view] || guides.overview;
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
  button.addEventListener("click", () => {
    const view = button.dataset.view || "overview";
    activateView(view);
    if (view === "maintenance" && !state.maintenanceOverview) void inspectMaintenance();
  });
});

document.querySelectorAll<HTMLButtonElement>("[data-maintenance-tab]").forEach((button) => {
  button.addEventListener("click", () => {
    const tab = button.dataset.maintenanceTab || "overview";
    document.querySelectorAll("[data-maintenance-tab]").forEach((item) => item.classList.toggle("active", item === button));
    document.querySelectorAll<HTMLElement>("[data-maintenance-panel]").forEach((panel) => panel.classList.toggle("active", panel.dataset.maintenancePanel === tab));
  });
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
document.querySelector("#export-doctor-json")?.addEventListener("click", async () => {
  try {
    if (!state.doctor) {
      state.doctor = await invoke<DoctorReport>("run_doctor");
      renderDoctor();
    }
    await runOperation(
      () => invoke<OperationResult>("export_doctor_report_json", { report: state.doctor! }),
      "正在导出 JSON 诊断报告",
    );
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
document.querySelector("#copy-doctor-report")?.addEventListener("click", async () => {
  try {
    if (!state.doctor) {
      state.doctor = await invoke<DoctorReport>("run_doctor");
      renderDoctor();
    }
    const text = await invoke<string>("doctor_report_text", { report: state.doctor!, format: "markdown" });
    await copyText(text);
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
  state.portHistory = await invoke<PortHistorySummary[]>("port_history");
  renderPorts();
});
document.querySelector("#discover-runtimes")?.addEventListener("click", async () => {
  state.runtimes = await invoke<RuntimeInfo[]>("discover_runtimes");
  renderRuntimes();
});
document.querySelector("#install-jdk")?.addEventListener("click", () => {
  const select = document.querySelector<HTMLSelectElement>("#jdk-version");
  const distribution = document.querySelector<HTMLSelectElement>("#jdk-distribution");
  if (!select || !distribution) return;
  void runRuntimeOperation(
    () => invoke<OperationResult>("install_jdk", { version: select.value, distribution: distribution.value }),
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
document.querySelector("#export-profiles")?.addEventListener("click", () => {
  void runOperation(() => invoke<OperationResult>("export_config_profiles"), "正在导出配置模板");
});
document.querySelector("#repair-doctor-safe")?.addEventListener("click", async () => {
  if (!window.confirm("将自动清理真实失效/重复 PATH，并修复 DevEnv 管理的用户级环境变量。不会安装软件、结束进程或修改系统级变量。确定继续吗？")) return;
  showToast("正在执行安全修复并重新诊断");
  try {
    const result = await invoke<DoctorRepairResult>("repair_doctor_safe");
    state.doctor = result.report;
    renderDoctor();
    const detail = result.applied.length ? result.applied.join("\n") : "没有可自动修复的安全项目";
    window.alert(`环境评分：${result.beforeScore} → ${result.afterScore}\n\n${detail}${result.remaining.length ? `\n\n仍需手动处理 ${result.remaining.length} 项。` : ""}`);
    showToast(`安全修复完成，当前评分 ${result.afterScore}`);
    await refreshBase();
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
document.querySelector("#profile-file-path")?.addEventListener("input", () => {
  state.profileImportPreview = null;
  renderProfileImportPreview();
});
document.querySelector("#preview-profiles")?.addEventListener("click", async () => {
  const path = document.querySelector<HTMLInputElement>("#profile-file-path")?.value.trim() || "";
  if (!path) {
    showToast("请输入团队模板 JSON 文件路径", true);
    return;
  }
  showToast("正在校验并预览配置模板");
  try {
    state.profileImportPreview = await invoke<ConfigProfileImportPreview>("preview_config_profiles", { path });
    renderProfileImportPreview();
    showToast(`模板预览完成，共 ${state.profileImportPreview.profiles.length} 个`);
  } catch (error) {
    state.profileImportPreview = null;
    renderProfileImportPreview();
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
document.querySelector("#import-profiles")?.addEventListener("click", () => {
  const path = document.querySelector<HTMLInputElement>("#profile-file-path")?.value.trim() || "";
  if (!path || !state.profileImportPreview) {
    showToast("请先预览并校验模板", true);
    return;
  }
  const replacements = state.profileImportPreview.profiles.filter((item) => item.willReplace).length;
  if (!window.confirm(`将导入 ${state.profileImportPreview.profiles.length} 个模板${replacements ? `，覆盖 ${replacements} 个同名模板` : ""}。确定继续吗？`)) return;
  void runOperation(() => invoke<OperationResult>("import_config_profiles", { path }), "正在导入配置模板").then(() => {
    state.profileImportPreview = null;
    renderProfileImportPreview();
  });
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
document.querySelector("#inspect-maintenance")?.addEventListener("click", () => void inspectMaintenance());
document.querySelector("#scan-maintenance")?.addEventListener("click", () => void scanMaintenance());
document.querySelector("#check-updates")?.addEventListener("click", async () => {
  showToast("正在检查新版本");
  try {
    state.update = await invoke<UpdateCheckResult>("check_for_updates");
    state.updateError = "";
    window.localStorage.setItem("devenv-last-update-check", String(Date.now()));
    renderUpdate();
    showToast(state.update.updateAvailable ? `发现新版本 ${state.update.latestVersion}` : "当前已是最新版本");
  } catch (error) {
    state.updateError = error instanceof Error ? error.message : String(error);
    renderUpdate();
    showToast(state.updateError, true);
  }
});
document.querySelector("#auto-check-updates")?.addEventListener("change", (event) => {
  const enabled = (event.target as HTMLInputElement).checked;
  void runOperation(
    () => invoke<ConfigView>("set_auto_check_update", { enabled }),
    enabled ? "正在启用启动更新检查" : "正在关闭启动更新检查",
  );
});
document.querySelector("#inspect-system-platforms")?.addEventListener("click", async () => {
  showToast("正在检查 Docker 与 WSL");
  try {
    state.systemPlatforms = await invoke<SystemPlatformReport>("inspect_system_platforms");
    renderSystemPlatforms();
    showToast("Docker 与 WSL 检查完成");
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
document.querySelector("#inspect-local-services")?.addEventListener("click", async () => {
  showToast("正在检查数据库与本地服务");
  try {
    state.localServices = await invoke<LocalServiceStatus[]>("inspect_local_services");
    renderLocalServices();
    showToast("本地服务检查完成");
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
document.querySelector("#open-docker-desktop")?.addEventListener("click", () => {
  void runOperation(() => invoke<OperationResult>("open_docker_desktop"), "正在启动 Docker Desktop");
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
  try {
    const assessment = await invoke<CommandSafetyAssessment>("inspect_command_safety", { command });
    if (!assessment.allowed) {
      throw new Error(`安全模式已拦截：${assessment.reason}`);
    }
    let confirmed = false;
    if (assessment.requiresConfirmation) {
      confirmed = window.confirm(`${assessment.reason}\n\n命令：${command}\n\n确定继续吗？`);
      if (!confirmed) return;
    }
    showToast(`正在运行白名单命令：${assessment.executable}`);
    const result = await invoke<CommandRunResult>("run_tool_command", {
      command,
      cwd: cwd || null,
      confirmed,
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
    await inspectProjectPorts(false);
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

document.querySelector("#inspect-project-ports")?.addEventListener("click", () => void inspectProjectPorts());
document.querySelector("#port-monitor-enabled")?.addEventListener("change", (event) => {
  const enabled = (event.target as HTMLInputElement).checked;
  if (portMonitorTimer !== null) {
    window.clearInterval(portMonitorTimer);
    portMonitorTimer = null;
  }
  if (enabled) {
    void pollPortMonitor(true);
    portMonitorTimer = window.setInterval(() => void pollPortMonitor(false), 5000);
    showToast("已开启常用端口占用提醒");
  } else {
    knownListeningPorts.clear();
    showToast("已关闭端口占用提醒");
  }
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
  if (action === "system-platform") {
    const platformAction = button.dataset.platformAction || "";
    const value =
      button.dataset.platformValue ||
      (platformAction === "wsl_install_distro"
        ? document.querySelector<HTMLInputElement>("#wsl-distro-name")?.value.trim()
        : undefined);
    const labels: Record<string, string> = {
      docker_install: "安装 Docker Desktop",
      docker_update: "升级 Docker Desktop",
      docker_shutdown: "退出 Docker Desktop",
      wsl_install: "安装 WSL",
      wsl_update: "更新 WSL",
      wsl_install_distro: `安装 WSL 发行版 ${value || ""}`,
      wsl_start: `启动 WSL 发行版 ${value || ""}`,
      wsl_terminate: `终止 WSL 发行版 ${value || ""}`,
      wsl_set_default: `将 ${value || ""} 设为默认发行版`,
    };
    if (!window.confirm(`${labels[platformAction] || "执行平台操作"}。需要管理员权限时 Windows 会显示 UAC，确定继续吗？`)) return;
    void runOperation(
      () => invoke<OperationResult>("manage_system_platform", { action: platformAction, value: value || null }),
      `正在${labels[platformAction] || "执行平台操作"}`,
    ).then(async () => {
      state.systemPlatforms = await invoke<SystemPlatformReport>("inspect_system_platforms");
      renderSystemPlatforms();
    });
  }
  if (action === "download-update") {
    void (async () => {
      if (!window.confirm("将从 GitHub Releases 下载新版安装包，并使用发布清单中的 SHA256 校验。确定继续吗？")) return;
      showToast("正在下载并校验更新安装包");
      try {
        const result = await invoke<OperationResult>("download_update");
        state.updateDownloaded = true;
        renderUpdate();
        showToast(result.message);
      } catch (error) {
        state.updateDownloaded = false;
        showToast(error instanceof Error ? error.message : String(error), true);
      }
    })();
  }
  if (action === "install-update") {
    if (!window.confirm("将启动已校验的安装器并退出当前程序。请保存正在进行的工作，确定继续吗？")) return;
    void (async () => {
      showToast("正在重新校验并启动更新安装器");
      try {
        await invoke<OperationResult>("launch_update_installer");
      } catch (error) {
        showToast(error instanceof Error ? error.message : String(error), true);
      }
    })();
  }
  if (action === "update-project-port") {
    const configId = button.dataset.configId || "";
    const config = state.projectPorts.find((item) => item.id === configId);
    const input = document.querySelector<HTMLInputElement>(`input[data-port-config-input="${configId}"]`);
    const newPort = Number(input?.value || 0);
    const path = document.querySelector<HTMLInputElement>("#project-path")?.value.trim() || "";
    if (!config || !Number.isInteger(newPort) || newPort < 1024 || newPort > 65535) {
      showToast("请输入 1024 到 65535 之间的有效端口", true);
      return;
    }
    if (!window.confirm(`将备份 ${config.file}，并把端口 ${config.currentPort} 修改为 ${newPort}。确定继续吗？`)) return;
    void runOperation(
      () => invoke<OperationResult>("update_project_port", { path, configId, newPort }),
      "正在备份并修改项目端口",
    ).then(() => void inspectProjectPorts(false));
  }
  if (action === "port-details") {
    const pid = Number(button.dataset.pid || 0);
    const port = Number(button.dataset.port || 0);
    state.selectedPort = state.ports.find((item) => item.pid === pid && item.localPort === port) || null;
    renderPortDetails();
  }
  if (action === "open-process-location") {
    const pid = Number(button.dataset.pid || 0);
    void runOperation(() => invoke<OperationResult>("open_process_location", { pid }), "正在打开进程位置");
  }
  if (action === "local-service-manage") {
    const serviceName = button.dataset.service || "";
    const serviceAction = button.dataset.serviceAction || "";
    const actionLabel = serviceAction === "start" ? "启动" : serviceAction === "stop" ? "停止" : "重启";
    if (!window.confirm(`将${actionLabel} Windows 服务 ${serviceName}。数据库连接可能短暂中断，确定继续吗？`)) return;
    void runOperation(
      () => invoke<OperationResult>("manage_local_service", { serviceName, action: serviceAction }),
      `正在${actionLabel}服务 ${serviceName}`,
    ).then(async () => {
      state.localServices = await invoke<LocalServiceStatus[]>("inspect_local_services");
      renderLocalServices();
    });
  }
  if (action === "local-service-logs") {
    const serviceName = button.dataset.service || "";
    const output = document.querySelector<HTMLElement>("#local-service-logs");
    if (output) output.textContent = `正在读取 ${serviceName} 最近 7 天日志...`;
    void invoke<string>("local_service_logs", { serviceName })
      .then((text) => {
        if (output) output.textContent = text;
      })
      .catch((error) => {
        if (output) output.textContent = error instanceof Error ? error.message : String(error);
      });
  }
  if (action === "local-service-directory") {
    const serviceName = button.dataset.service || "";
    void runOperation(
      () => invoke<OperationResult>("open_local_service_directory", { serviceName }),
      `正在打开 ${serviceName} 程序目录`,
    );
  }
  if (action === "stop-local-service") {
    const port = Number(button.dataset.port || 0);
    const serviceName = button.dataset.service || "";
    const ok = window.confirm(`将停止 Windows 服务 ${serviceName}（端口 ${port}）。这会中断当前数据库连接，确定继续吗？`);
    if (!ok) return;
    void runOperation(
      () => invoke<OperationResult>("stop_local_service", { port, serviceName }),
      `正在停止服务 ${serviceName}`,
    ).then(async () => {
      state.localServices = await invoke<LocalServiceStatus[]>("inspect_local_services");
      renderLocalServices();
    });
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
    const currentHome = state.javaEnvironment?.javaHome || state.env?.javaHome || "未设置";
    const targetHome = path || `%DEVENV_HOME%\\current\\jdk（JDK ${version}）`;
    if (!window.confirm(`将修改当前用户的 JDK 生效链：\n\nJAVA_HOME：${currentHome}\n→ ${targetHome}\n\n受管 PATH 中的 JDK 会保持在首位；切换后将自动验证 java、javac、Maven 与 Gradle。确定继续吗？`)) return;
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
    const source = button.dataset.source || "未知来源";
    const method = source === "Scoop" || source === "Chocolatey" ? `调用 ${source} 的卸载流程` : "启动匹配的 Windows 卸载器";
    const ok = window.confirm(`来源：${source}\n将${method}卸载 ${kind}。不会直接删除包管理器目录。\n\n${executable}\n\n确定继续吗？`);
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
  if (action === "install-apply-profile") {
    const id = button.dataset.id || "";
    void (async () => {
      try {
        const requirements = await invoke<ProfileRequirement[]>("config_profile_requirements", { id });
        const missing = requirements.filter((item) => !item.installed);
        const message = missing.length
          ? `将联网安装：${missing.map((item) => `${item.kind} ${item.version}`).join("、")}，安装完成后应用模板。确定继续吗？`
          : "所需运行时均已安装，将直接应用模板。确定继续吗？";
        if (!window.confirm(message)) return;
        await runRuntimeOperation(
          () => invoke<OperationResult>("install_profile_missing", { id }),
          missing.length ? "正在补齐模板所需运行时" : "正在应用配置模板",
          "PATH",
        );
      } catch (error) {
        showToast(error instanceof Error ? error.message : String(error), true);
      }
    })();
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
    void terminatePortProcess(pid);
  }
});

void listen<TaskProgress>("task-progress", (event) => renderProgress(event.payload));
void refreshAll(false).then(() => {
  window.setTimeout(() => void refreshRuntimeAndPorts(true), 350);
  window.setInterval(async () => {
    try {
      state.runtimes = await invoke<RuntimeInfo[]>("discover_runtimes");
      renderRuntimes();
    } catch {
      // 实时版本刷新保持静默。
    }
  }, 30_000);
  if (state.config?.settings.autoCheckUpdate) {
    const lastCheck = Number(window.localStorage.getItem("devenv-last-update-check") || 0);
    if (Date.now() - lastCheck >= 24 * 60 * 60 * 1000) {
      window.setTimeout(async () => {
        try {
          state.update = await invoke<UpdateCheckResult>("check_for_updates");
          state.updateError = "";
          window.localStorage.setItem("devenv-last-update-check", String(Date.now()));
          renderUpdate();
          if (state.update.updateAvailable) showToast(`发现新版本 ${state.update.latestVersion}`);
        } catch (error) {
          state.updateError = error instanceof Error ? error.message : String(error);
          renderUpdate();
        }
      }, 1500);
    }
  }
});
document.querySelector("#inspect-java")?.addEventListener("click", () => void inspectJava());
document.querySelector("#inspect-agent-traces")?.addEventListener("click", async () => {
  const projectPath = document.querySelector<HTMLInputElement>("#project-path")?.value.trim() || null;
  showToast("正在进行本地只读痕迹分析");
  try {
    state.agentTraces = await invoke<AgentTraceReport>("inspect_agent_traces", { projectPath });
    renderAgentTraces();
    showToast(`分析完成，发现 ${state.agentTraces.items.length} 条可验证线索`);
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
