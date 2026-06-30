import { invoke, listen, open } from "./api/tauri";
import {
  Activity,
  Boxes,
  Clipboard,
  Cpu,
  Database,
  Download,
  FileText,
  FolderOpen,
  FolderSearch,
  Gauge,
  Hammer,
  KeyRound,
  Network,
  PackageCheck,
  Play,
  RefreshCw,
  RotateCcw,
  Route,
  Search,
  Shield,
  Terminal,
  Trash2,
  type IconNode,
} from "lucide";
import { envReliabilityIntro } from "./envReliability";
import { askForConfirmation, confirmRisk, disclaimerPanel } from "./features/safety";
import { fileDirectory } from "./features/cleanup";
import { clearFeatureHelp, renderViewGuide } from "./features/help";
import { projectConfigurationPlanId } from "./features/jdk";
import { MYSQL_PERMISSION_UNKNOWN_HELP, mysqlPathValue } from "./features/mysql";
import { canShowKillPortAction } from "./features/ports";
import { SAFE_MODE_DESCRIPTION } from "./features/safeMode";
import { hideToast, showToast } from "./features/toast";
import { updateEmptyState } from "./features/update";
import { riskBadge } from "./components/riskBadge";
import type {
  AppSnapshot,
  EnvSnapshot,
  ConfigView,
  ManagedRuntime,
  OperationResult,
  ToolProbe,
  EnvRepairAction,
  EnvRepairPlan,
  EnvRepairResult,
  EnvBackupRecord,
  EnvReliabilitySnapshot,
  FeatureRiskInfo,
  ValidationCheck,
  PythonIntegrityReport,
  RuntimeStrongVerificationReport,
  IdeaProjectReport,
  JavaConsumerReport,
  KillResult,
  RuntimeInfo,
  JavaEnvironmentReport,
  PortRecord,
  PortHistorySummary,
  PortSortKey,
  SortDirection,
  ProjectHealth,
  TaskProgress,
  NetworkDiagnostics,
  CacheEntry,
  CommandRunResult,
  CommandSafetyAssessment,
  AgentTraceReport,
  EnvHealthCheck,
  ConfigProfile,
  DoctorReport,
  PythonAnalysis,
  PythonRepairPlan,
  PythonToolState,
  PythonEntry,
  ProjectAnalysis,
  CurrentVersions,
  ProjectConfigFileDraft,
  ProjectConfigPreview,
  ProjectPortConfig,
  ToolState,
  ToolchainReport,
  PlatformReport,
  SystemPlatformReport,
  LocalServiceStatus,
  MySqlCandidate,
  MySqlBackupManifestStatus,
  MySqlRepairReport,
  MySqlRepairPlan,
  ConfirmationTokenView,
  MySqlExecutionGuard,
  JdkDistribution,
  UpdateCheckResult,
  CleanupArchitecture,
  DoctorRepairResult,
  ConfigProfileImportPreview,
  ProfileRequirement,
  CleanupCandidate,
  CleanupCategoryScan,
  CleanupScanReport,
  CleanupPlan,
  CleanupResult,
  MovePlan,
  MoveResult,
  RollbackRecord,
  PartitionInfo,
  PartitionLayoutReport,
  ExpansionPlan,
  ExpansionResult,
  DiskVolumeInfo,
  MaintenanceOverview,
  LargeFileItem,
  ArchivePlanItem,
  DuplicateGroup,
  FolderUsageReport,
  InstalledSoftwareUsage,
  AppUsageItem,
  AppUsageReport,
  EnvironmentConfigPreview,
  EnvironmentBackupInfo,
} from "./types";
import "./styles.css";

const app = document.querySelector<HTMLDivElement>("#app");
const SAFETY_DISCLAIMER_VERSION = 1;

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
        <span class="nav-group">诊断</span>
        <button class="nav-item active" data-view="overview">${icon(Gauge)}<span>总览</span></button>
        <button class="nav-item" data-view="doctor">${icon(Shield)}<span>环境医生</span></button>
        <button class="nav-item" data-view="ports">${icon(Network)}<span>端口</span></button>
        <span class="nav-group">环境与运行时</span>
        <button class="nav-item" data-view="runtimes">${icon(Terminal)}<span>版本管理</span></button>
        <button class="nav-item" data-view="environment">${icon(Route)}<span>环境</span></button>
        <span class="nav-group">项目与生态</span>
        <button class="nav-item" data-view="project">${icon(FolderSearch)}<span>项目</span></button>
        <button class="nav-item" data-view="toolchains">${icon(PackageCheck)}<span>工具链</span></button>
        <button class="nav-item" data-view="platforms">${icon(Cpu)}<span>平台/镜像</span></button>
        <button class="nav-item" data-view="learning">${icon(FileText)}<span>学习中心</span></button>
        <span class="nav-group">维护与系统</span>
        <button class="nav-item" data-view="maintenance">${icon(Shield)}<span>空间分析</span></button>
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
      <div id="fatal-error" class="fatal-error" hidden></div>
      <div id="safety-gate" class="safety-gate" hidden></div>
      <div id="task-progress" class="task-progress" hidden>
        <div><strong id="task-progress-title">任务</strong><span id="task-progress-message">等待中</span></div>
        <div class="progress-track"><span id="task-progress-bar"></span></div>
      </div>
      <details id="view-guide" class="view-guide">
        <summary>${icon(FileText)}<span>页面使用指南</span></summary>
        <div id="view-guide-text" class="view-guide-body">先看系统快照和当前生效工具；需要深入排查时再进入环境医生。</div>
      </details>
      <div id="feature-help-slot"></div>

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
            <div class="panel-head"><div class="panel-title">${icon(RefreshCw)}<h2>版本更新</h2></div><button id="check-updates-overview" data-action="check-updates">${icon(RefreshCw)}<span>检查更新</span></button></div>
            <div id="overview-update-result" data-update-result><div class="empty">尚未检查新版本</div></div>
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
        <section class="panel recommendation-panel">
          <div class="panel-head"><div class="panel-title">${icon(PackageCheck)}<h2>成熟开源工具推荐</h2></div><button data-action="open-learning">查看使用边界</button></div>
          <div class="recommendation-grid">
            ${[
              ["Scoop", "Windows 命令行安装器", "https://scoop.sh/"],
              ["mise", "多语言版本管理", "https://mise.en.dev/"],
              ["vfox", "跨平台 SDK 版本管理", "https://vfox.dev/"],
              ["uv", "高速 Python 项目与包管理", "https://docs.astral.sh/uv/"],
              ["chsrc", "多生态换源工具", "https://github.com/RubyMetric/chsrc"],
            ].map(([name, summary, url]) => `<article class="recommendation-card"><strong>${name}</strong><span>${summary}</span><button data-action="copy-text" data-copy="${url}">复制官网</button></article>`).join("")}
          </div>
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
          <div id="doctor-repair-result" class="runtime-list"></div>
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
          <div id="port-summary" class="port-summary"></div>
          <div class="port-tools">
            <div class="search-box">${icon(Search)}<input id="port-search" placeholder="搜索端口、进程、PID、服务名或识别结果" /></div>
            <label class="toggle-row port-monitor-toggle" title="每 5 秒检查新出现的常用监听端口">
              <input id="port-monitor-enabled" type="checkbox" />
              <span>占用提醒</span>
            </label>
            <div id="port-quick-filters" class="chip-row port-filter-row">
              <button class="filter-chip active" data-port-filter="all">全部</button>
              <button class="filter-chip" data-port-filter="listening">监听中</button>
              <button class="filter-chip" data-port-filter="development">开发服务</button>
              <button class="filter-chip" data-port-filter="frontend">前端</button>
              <button class="filter-chip" data-port-filter="backend">后端</button>
              <button class="filter-chip" data-port-filter="python">Python/AI</button>
              <button class="filter-chip" data-port-filter="database">数据库</button>
              <button class="filter-chip" data-port-filter="middleware">中间件</button>
              <button class="filter-chip" data-port-filter="desktop">桌面应用</button>
              <button class="filter-chip" data-port-filter="sensitive">系统/高风险</button>
              <button class="filter-chip" data-port-filter="low-confidence">低置信度</button>
            </div>
          </div>
          <div class="port-workbench">
            <div class="table-wrap">
              <table class="ports-table">
                <colgroup>
                  <col class="col-port" />
                  <col class="col-state" />
                  <col class="col-identity" />
                  <col class="col-process" />
                  <col class="col-pid" />
                  <col class="col-confidence" />
                  <col class="col-risk" />
                  <col class="col-action" />
                </colgroup>
                <thead>
                  <tr>
                    <th><button class="sort-head" data-sort="localPort">端口</button></th>
                    <th><button class="sort-head" data-sort="state">状态</button></th>
                    <th><button class="sort-head" data-sort="identity">识别结果</button></th>
                    <th><button class="sort-head" data-sort="processName">进程</button></th>
                    <th><button class="sort-head" data-sort="pid">PID</button></th>
                    <th><button class="sort-head" data-sort="confidence">置信度</button></th>
                    <th><button class="sort-head" data-sort="riskLevel">风险</button></th>
                    <th>更多</th>
                  </tr>
                </thead>
                <tbody id="ports-body"></tbody>
              </table>
            </div>
            <aside id="port-detail" class="port-detail"><div class="empty">点击端口行查看详情</div></aside>
          </div>
          <div id="ports-pagination"></div>
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
            <div class="toolbar compact">
              <button id="analyze-python">${icon(Search)}<span>分析</span></button>
              <button id="inspect-python-integrity">${icon(Shield)}<span>完整性检查</span></button>
            </div>
          </div>
          <div id="python-analysis" class="python-analysis"></div>
          <div id="python-integrity-result" class="runtime-list"><div class="empty">点击“完整性检查”验证 pip、venv、ssl、sqlite3、ctypes 和 tkinter。</div></div>
          <div class="repair-options">
            <label><input id="python-repair-pip" type="checkbox" checked /> 修复并验证当前 Python 的 pip</label>
            <label><input id="python-repair-path" type="checkbox" checked /> 将当前 Python/Scripts 置于用户 PATH 前部</label>
            <button id="preview-python-repair">生成可审计修复计划</button>
          </div>
          <div id="python-repair-preview"><div class="empty">先分析，再预览；不会卸载其他 Python 或自动关闭 Store 别名</div></div>
        </section>
        <section class="panel runtime-manager">
          <div class="panel-head">
            <div class="panel-title">${icon(Download)}<h2>构建工具</h2></div>
            <button id="inspect-runtime-strong">${icon(Shield)}<span>强验证所有运行时</span></button>
          </div>
          <div class="toolbar">
            <button id="install-maven">${icon(Download)}<span>安装 Maven 最新版</span></button>
            <button id="install-gradle">${icon(Download)}<span>安装 Gradle 最新版</span></button>
          </div>
          <div id="managed-build-tools" class="runtime-list"></div>
          <div id="runtime-strong-result" class="runtime-list"><div class="empty">检查 JDK/Python/Node/Maven/Gradle/Go 的登记、组件、current 指针和环境生效状态。</div></div>
        </section>
      </section>

      <section id="view-environment" class="view">
        <div id="safety-disclaimer-slot"></div>
        <div class="grid two">
          <section class="panel env-reliability-panel">
            <div class="panel-head">
              <div class="panel-title">${icon(Shield)}<h2>环境可靠性中心</h2></div>
              <div class="toolbar compact">
                <button id="inspect-env-reliability" class="primary">${icon(Activity)}<span>检查可靠性</span></button>
                <button id="export-env-reliability">${icon(FileText)}<span>导出报告</span></button>
              </div>
            </div>
            <div class="scan-only-banner">${icon(Shield)}<span>只读检查会同时比较当前进程环境和用户环境；修复计划只修改当前用户级环境变量。</span></div>
            <div id="env-reliability-result"><div class="empty">点击“检查可靠性”查看 JAVA_HOME raw/expanded、PATH 命中顺序、Python/pip、Maven/Gradle 和 Nacos 风险。</div></div>
            <div class="form-row">
              <input id="java-stabilize-path" placeholder="JDK 根目录，例如 D:\\DevEnvManager\\current\\jdk" />
              <button data-pick-directory="java-stabilize-path">${icon(FolderSearch)}<span>选择 JDK</span></button>
              <button id="create-java-stabilize-plan">生成 Java 稳定修复计划</button>
              <button id="apply-env-repair-plan" class="danger-button" disabled>确认写入用户级 JAVA_HOME/PATH</button>
            </div>
            <div id="env-repair-plan-result"><div class="empty">修复计划会展示 diff、备份名、风险说明和验证结果。</div></div>
            <div class="panel-head compact-title"><div class="panel-title">${icon(RefreshCw)}<h3>Phase 5 环境备份中心</h3></div><button id="load-env-backup-records">刷新备份记录</button></div>
            <div id="env-backup-records" class="runtime-list"><div class="empty">尚未读取 Phase 5 环境备份</div></div>
          </section>
          <section class="panel">
            <div class="panel-title">${icon(Route)}<h2>环境变量</h2></div>
            <div class="toolbar">
              <button id="configure-env">${icon(Shield)}<span>预览配置</span></button>
              <button id="check-env-health">${icon(Activity)}<span>检查</span></button>
              <button id="cleanup-path">${icon(Trash2)}<span>清理失效 PATH</span></button>
              <button id="restore-env">${icon(RefreshCw)}<span>恢复</span></button>
            </div>
            <div id="env-list" class="kv-list"></div>
            <div id="env-health" class="runtime-list health-list"></div>
            <div id="env-config-preview"><div class="empty">点击“预览配置”查看 DEVENV_HOME、JAVA_HOME 和 PATH 的实际差异</div></div>
            <div class="panel-head compact-title"><div class="panel-title">${icon(RefreshCw)}<h3>环境备份历史</h3></div><button id="load-env-backups">刷新备份</button></div>
            <div id="env-backup-list" class="runtime-list"><div class="empty">尚未读取环境备份</div></div>
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
        <section class="panel platform-section chsrc-section">
          <div class="panel-head">
            <div class="panel-title">${icon(RefreshCw)}<h2>chsrc 统一换源</h2></div>
            <span id="chsrc-status" class="risk-chip">尚未检测</span>
          </div>
          <p class="small-note">调用官方 RubyMetric/chsrc，不在程序内重新实现换源；只接受固定目标和 chsrc 列出的源 ID，不接受自定义 URL。</p>
          <div class="form-row">
            <select id="chsrc-target">
              <option value="node">Node.js</option><option value="python">Python</option>
              <option value="go">Go</option><option value="rust">Rust</option>
              <option value="cargo">Cargo</option><option value="maven">Maven</option>
              <option value="gradle">Gradle</option><option value="nuget">NuGet</option>
            </select>
            <input id="chsrc-source" placeholder="源 ID，例如 tuna；自动测速可留空" />
          </div>
          <div class="toolbar">
            <button data-chsrc-action="get">查看当前源</button>
            <button data-chsrc-action="list">列出可用源</button>
            <button data-chsrc-action="measure">测速</button>
            <button data-chsrc-action="auto" class="primary">自动选择</button>
            <button data-chsrc-action="set">使用源 ID</button>
            <button data-chsrc-action="reset">恢复官方源</button>
          </div>
          <div id="chsrc-recovery" class="notice-panel"></div>
          <pre id="chsrc-output" class="command-output compact-output">安装提示：scoop install chsrc，或通过 WinGet 安装 RubyMetric/chsrc。</pre>
        </section>
      </section>

      <section id="view-learning" class="view learning-view">
        <section class="panel learning-hero">
          <div class="panel-title">${icon(FileText)}<h2>环境配置学习中心</h2></div>
          <p>这里解释工具边界、安装来源和检查命令。学习中心只运行固定的只读检测命令，不安装软件、不写环境变量；配置仍由你在对应页面预览和确认。</p>
        </section>
        <section class="panel">
          <div class="panel-title">${icon(Terminal)}<h2>只读命令练习区</h2></div>
          <p class="small-note">支持：java/javac、python/pip/py、node/npm、mvn、gradle、go、rustc/cargo、dotnet 的版本与位置检查。安装、set、config、删除和 Shell 命令会被后端拒绝。</p>
          <div class="form-row wide"><input id="learning-command" value="python --version" placeholder="输入只读检测命令，例如 python -m pip --version" /><button id="run-learning-command" class="primary">安全检查并运行</button></div>
          <pre id="learning-output" class="command-output">命令不会自动配置环境；请先理解输出，再前往环境或版本管理页面操作。</pre>
        </section>
        <section class="panel">
          <div class="panel-title">${icon(PackageCheck)}<h2>推荐工具与适用边界</h2></div>
          <div class="learning-cards">
            ${[
              ["Scoop", "https://scoop.sh/", "适合安装 Windows CLI 工具；软件归属仍由 Scoop 管理。", "scoop --version"],
              ["mise", "https://mise.en.dev/", "适合项目级多语言版本；不要和多个版本管理器同时接管同一 PATH。", "mise doctor"],
              ["vfox", "https://vfox.dev/", "适合通过插件管理 SDK；先检查插件来源和项目配置。", "vfox version"],
              ["uv", "https://docs.astral.sh/uv/", "适合 Python 项目、虚拟环境和依赖；全局 Python 仍需明确来源。", "uv --version"],
              ["chsrc", "https://github.com/RubyMetric/chsrc", "只负责查看、测速和切换软件源，不负责安装运行时。", "chsrc --version"],
            ].map(([name, url, boundary, command]) => `<article class="learning-card"><div><strong>${name}</strong><span>${boundary}</span></div><code>${command}</code><div class="row-actions"><button data-action="copy-text" data-copy="${url}">复制官网</button><button data-action="copy-text" data-copy="${command}">复制检查命令</button></div></article>`).join("")}
          </div>
        </section>
        <section class="panel">
          <div class="panel-title">${icon(Route)}<h2>推荐学习顺序</h2></div>
          <ol class="learning-steps"><li>用 <code>where.exe</code> 与版本命令确认当前真正生效的程序。</li><li>在“环境医生”查看 PATH、JAVA_HOME、pip 归属和多版本冲突。</li><li>选择一个主要版本管理方案，避免 Scoop、mise、vfox、手工 PATH 同时接管同一工具。</li><li>任何配置先预览和备份；完成后重新打开终端，再次运行只读命令验证。</li></ol>
        </section>
      </section>

      <section id="view-maintenance" class="view maintenance-view">
        <section class="maintenance-hero">
          <div>
            <span class="phase-badge">只读分析与安全清理</span>
            <h2>开发环境空间分析</h2>
            <p>严格执行扫描 → 选择 → 计划预览 → 二次确认 → 清理 → 验证 → 报告。普通文件优先移入 Windows 回收站。</p>
          </div>
          <div class="toolbar compact">
            <button id="inspect-maintenance" class="primary">${icon(Activity)}<span>开始体检</span></button>
            <button id="scan-maintenance">${icon(Search)}<span>安全扫描</span></button>
          </div>
        </section>
        <nav class="maintenance-tabs" aria-label="空间分析功能">
          <button class="active" data-maintenance-tab="overview">总览</button>
          <button data-maintenance-tab="cleanup">C盘专清</button>
          <button data-maintenance-tab="dev-cache">开发缓存</button>
          <button data-maintenance-tab="desktop">桌面急救</button>
          <button data-maintenance-tab="downloads">下载目录</button>
          <button data-maintenance-tab="large-files">大文件</button>
          <button data-maintenance-tab="duplicates">重复文件</button>
          <button data-maintenance-tab="apps">软件与应用</button>
          <button data-maintenance-tab="move">空间搬家</button>
          <button data-maintenance-tab="report">报告</button>
        </nav>

        <section class="maintenance-panel active" data-maintenance-panel="overview">
          <div id="maintenance-overview"><div class="empty">点击“开始体检”读取 C 盘与各分区容量</div></div>
        </section>
        <section class="maintenance-panel" data-maintenance-panel="cleanup">
          <div class="scan-only-banner">${icon(Shield)}<span>Windows Temp、回收站、Windows Update、Windows.old、个人目录、浏览器、微信/QQ 与系统缓存始终不会自动清理。</span></div>
          <div class="toolbar cleanup-modes">
            <button id="select-conservative">保守清理</button>
            <button id="select-recommended">推荐清理</button>
            <button id="clear-cleanup-selection">清空选择</button>
            <button id="preview-cleanup-plan" class="primary" disabled>预览清理计划</button>
          </div>
          <div id="maintenance-cleanup-categories"><div class="empty">点击“安全扫描”生成真实扫描结果</div></div>
          <details class="expert-scan">
            <summary>专家扫描（默认折叠）</summary>
            <p class="small-note">展示所有高风险与只读统计项。它们不会进入自动清理计划。</p>
            <div id="maintenance-expert-categories"><div class="empty">扫描后显示系统与高风险分类</div></div>
          </details>
          <section id="cleanup-plan-preview" class="panel cleanup-plan-panel"><div class="empty">选择项目后点击“预览清理计划”</div></section>
          <button id="execute-cleanup-plan" class="danger-button" disabled>${icon(Trash2)}<span>确认执行清理</span></button>
        </section>
        <section class="maintenance-panel" data-maintenance-panel="dev-cache">
          <div class="scan-only-banner">${icon(Shield)}<span>开发缓存优先调用官方命令。Maven、Gradle、Cargo 和项目 target 只扫描，不默认清理。</span></div>
          <div class="dev-cache-actions">
            ${[
              ["npm", "npm cache clean --force"], ["pnpm", "pnpm store prune"], ["yarn", "yarn cache clean"],
              ["pip", "python -m pip cache purge"], ["uv", "uv cache clean"], ["poetry", "poetry cache clear pypi --all"],
              ["go-cache", "go clean -cache"], ["go-modcache", "go clean -modcache"], ["dotnet", "dotnet nuget locals all --clear"],
            ].map(([tool, command]) => `<button data-dev-cache="${tool}" title="${command}">${escapeHtml(command)}</button>`).join("")}
          </div>
          <div id="maintenance-dev-categories"><div class="empty">点击“安全扫描”统计开发缓存</div></div>
        </section>
        <section class="maintenance-panel" data-maintenance-panel="desktop">
          <div class="panel-head"><div class="panel-title">${icon(FolderSearch)}<h2>桌面急救</h2></div><button id="inspect-desktop">只读分析桌面</button></div>
          <div class="scan-only-banner">${icon(Shield)}<span>只统计占用并生成整理建议；不删除、不移动桌面文件。归档整理必须先预览计划并再次确认。</span></div>
          <div id="desktop-usage"><div class="empty">尚未分析桌面</div></div>
        </section>
        <section class="maintenance-panel" data-maintenance-panel="downloads">
          <div class="panel-head"><div class="panel-title">${icon(Download)}<h2>下载目录</h2></div><button id="inspect-downloads">只读分析下载目录</button></div>
          <div class="scan-only-banner">${icon(Shield)}<span>分类安装包、压缩包、视频、图片、文档、镜像、旧文件和大文件；本阶段不移动、不删除。</span></div>
          <div id="downloads-usage"><div class="empty">尚未分析下载目录</div></div>
        </section>
        <section class="maintenance-panel" data-maintenance-panel="large-files">
          <div class="panel-head"><div class="panel-title">${icon(Search)}<h2>大文件 Top 100</h2></div><div class="row-actions"><button id="scan-large-files">开始只读扫描</button><button id="cancel-large-scan" disabled>取消扫描</button></div></div>
          <div class="form-row"><input id="large-file-root" placeholder="扫描目录；留空使用用户目录" /><button data-pick-directory="large-file-root">${icon(FolderSearch)}<span>选择文件夹</span></button><input id="large-file-min" type="number" min="1" value="100" title="最小 MB" /><span>MB 以上</span></div>
          <div id="large-file-result" class="runtime-list"><div class="empty">选择或填写扫描范围后开始</div></div>
        </section>
        <section class="maintenance-panel" data-maintenance-panel="duplicates">
          <div class="panel-head"><div class="panel-title">${icon(Boxes)}<h2>重复文件</h2></div><div class="row-actions"><button id="scan-duplicates">按大小与 SHA256 扫描</button><button id="cancel-duplicate-scan" disabled>取消扫描</button></div></div>
          <div class="scan-only-banner">${icon(Shield)}<span>只扫描用户明确选择的目录；先按大小分组，再读取候选内容计算 SHA256。不提供删除。</span></div>
          <div class="form-row"><input id="duplicate-root" placeholder="扫描目录；留空使用用户目录" /><button data-pick-directory="duplicate-root">${icon(FolderSearch)}<span>选择文件夹</span></button><input id="duplicate-min" type="number" min="1" value="10" title="最小 MB" /><span>MB 以上</span></div>
          <div id="duplicate-result"><div class="empty">尚未扫描重复文件</div></div>
        </section>
        <section class="maintenance-panel" data-maintenance-panel="apps">
          <div class="panel-head"><div class="panel-title">${icon(Database)}<h2>软件与常见应用占用</h2></div><button id="inspect-app-usage">开始只读分析</button></div>
          <div class="scan-only-banner">${icon(Shield)}<span>微信/QQ 不读取聊天数据库；浏览器不扫描 Cookie、密码和登录态；软件与游戏不直接删除安装目录。</span></div>
          <div id="app-usage-result"><div class="empty">尚未分析常见应用与已安装软件</div></div>
        </section>
        <section class="maintenance-panel" data-maintenance-panel="move">
          <div class="panel-head"><div class="panel-title">${icon(Boxes)}<h2>空间搬家与归档</h2></div><button id="load-archive-plan">刷新计划</button></div>
          <div class="scan-only-banner">${icon(Shield)}<span>支持桌面/下载归档、白名单目录搬家和 Junction 桥接；执行前必须预览计划并二次确认。</span></div>
          <div class="form-row">
            <input id="move-source" placeholder="源目录，例如 C:\\Users\\你\\Downloads 或缓存目录" />
            <input id="move-target-drive" value="D:" placeholder="目标盘或目标目录，例如 D:" />
            <button data-pick-directory="move-source">${icon(FolderSearch)}<span>源目录</span></button>
            <button data-pick-directory="move-target-drive">${icon(FolderSearch)}<span>目标目录</span></button>
            <select id="move-mode">
              <option value="archive_only">归档整理</option>
              <option value="move_cache_folder">缓存搬家</option>
              <option value="move_user_folder">用户目录搬家</option>
              <option value="junction_bridge">Junction 桥接</option>
            </select>
            <button id="preview-move-plan">生成搬家计划</button>
          </div>
          <div class="toolbar compact">
            <button id="preview-desktop-archive">桌面归档计划</button>
            <button id="preview-downloads-archive">下载归档计划</button>
            <button id="execute-move-plan" class="danger-button" disabled>二次确认后执行</button>
          </div>
          <div id="move-plan-result" class="runtime-list"><div class="empty">还没有空间搬家计划</div></div>
          <div id="archive-plan-list" class="runtime-list"><div class="empty">可从大文件或重复文件结果加入归档计划</div></div>
          <div class="panel-head"><div class="panel-title">${icon(RefreshCw)}<h3>回滚记录</h3></div><button id="load-rollback-records">刷新回滚</button></div>
          <div id="rollback-records" class="runtime-list"><div class="empty">暂无可自动回滚记录</div></div>
        </section>
        <section class="maintenance-panel" data-maintenance-panel="report">
          <div class="panel maintenance-placeholder">
            <h2>报告</h2>
            <p>清理完成后显示释放空间、跳过项、失败项和可导出的 Markdown / JSON 报告。</p>
            <div id="cleanup-report"><div class="empty">还没有清理报告</div></div>
          </div>
        </section>
      </section>

      <section id="view-toolbox" class="view">
        <div class="grid two">
          <section class="panel">
            <div class="panel-head">
              <div class="panel-title">${icon(Boxes)}<h2>Docker / WSL</h2></div>
              <button id="inspect-system-platforms">${icon(RefreshCw)}<span>检查</span></button>
            </div>
            <div id="system-platform-result" class="platform-content"><div class="empty">尚未检查 Docker 与 WSL</div></div>
            <details class="advanced-tools">
              <summary>${icon(Shield)}<span>高级 / 系统工具</span></summary>
              <p class="small-note">这些入口可能安装、更新、退出系统级组件或触发 UAC；只在明确知道影响范围时使用。</p>
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
            </details>
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
        <section class="panel mysql-repair-panel">
          <div class="panel-head"><div class="panel-title">${icon(Database)}<h2>MySQL 修复中心</h2></div><button id="inspect-mysql-repair" class="primary">只读深度诊断</button></div>
          <div class="advanced-warning">${icon(Shield)}<span><strong>先诊断、再计划、再确认。</strong> 不读取表内容或密码；补回系统库前必须先由本程序完成完整 Data 备份，且永不覆盖业务库、ibdata1 或 ib_logfile。</span></div>
          <div id="mysql-repair-result"><div class="empty">检查服务丢失、1067 线索、my.ini、Data 系统库和候选业务库</div></div>
          <section id="mysql-plan-preview" class="repair-plan"><div class="empty">选择候选与动作后显示一次性修复计划</div></section>
        </section>
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
            <button data-pick-directory="command-cwd">${icon(FolderSearch)}<span>选择文件夹</span></button>
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
          <div id="update-result" data-update-result><div class="empty">尚未检查新版本</div></div>
        </section>
        <details class="panel runtime-manager danger-panel advanced-tools">
          <summary>${icon(Trash2)}<span>高级 / 卸载本程序</span></summary>
          <div class="toolbar">
            <button id="self-uninstall" class="danger-button">${icon(Trash2)}<span>启动卸载程序</span></button>
          </div>
          <div class="small-note">只打开 Windows 卸载器并关闭当前程序；不会自行删除用户项目、数据库或运行时目录。</div>
        </details>
      </section>

      <section id="view-project" class="view">
        <section class="panel">
          <div class="panel-title">${icon(FolderSearch)}<h2>项目启动向导</h2></div>
          <div class="form-row">
            <input id="project-path" placeholder="选择你的项目目录" />
            <button data-pick-directory="project-path" data-auto-analyze="true">${icon(FolderSearch)}<span>选择文件夹</span></button>
            <button id="check-project">${icon(Play)}<span>分析</span></button>
          </div>
          <div class="toolbar project-actions">
            <button id="preview-project-config">${icon(Hammer)}<span>生成 VS Code / IDEA 配置预览</span></button>
            <button id="inspect-idea-project">${icon(Search)}<span>只读读取 IDEA 配置</span></button>
            <button id="verify-nacos-java">${icon(Shield)}<span>验证 Nacos Java</span></button>
            <button id="verify-nexus-java">${icon(Shield)}<span>验证 Nexus Java</span></button>
          </div>
          <section id="project-config-preview" class="project-config-preview"><div class="empty">先分析项目，再生成可编辑的配置预览</div></section>
          <section id="idea-project-result" class="runtime-list"><div class="empty">选择项目文件夹后，可只读分析 .idea/misc.xml、compiler.xml 和 *.iml。</div></section>
          <section id="java-consumer-result" class="runtime-list"><div class="empty">Nacos/Nexus/Maven/Gradle/Spring Boot 等 Java 消费者验证会读取最新用户环境，不修改项目。</div></section>
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
  environmentPreview: null as EnvironmentConfigPreview | null,
  environmentBackups: [] as EnvironmentBackupInfo[],
  envReliability: null as EnvReliabilitySnapshot | null,
  envRepairPlan: null as EnvRepairPlan | null,
  envRepairResult: null as EnvRepairResult | null,
  envBackupRecords: [] as EnvBackupRecord[],
  safetyDisclaimer: "",
  featureRisks: [] as FeatureRiskInfo[],
  config: null as ConfigView | null,
  runtimes: [] as RuntimeInfo[],
  javaEnvironment: null as JavaEnvironmentReport | null,
  externalJdkChecks: {} as Record<string, ValidationCheck[]>,
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
  pythonIntegrity: null as PythonIntegrityReport | null,
  pythonRepairPlan: null as PythonRepairPlan | null,
  runtimeStrong: null as RuntimeStrongVerificationReport | null,
  project: null as ProjectAnalysis | null,
  ideaProject: null as IdeaProjectReport | null,
  javaConsumer: null as JavaConsumerReport | null,
  projectConfigPreview: null as ProjectConfigPreview | null,
  toolchains: null as ToolchainReport | null,
  platforms: null as PlatformReport | null,
  projectPorts: [] as ProjectPortConfig[],
  systemPlatforms: null as SystemPlatformReport | null,
  localServices: [] as LocalServiceStatus[],
  mysqlRepair: null as MySqlRepairReport | null,
  mysqlPlan: null as MySqlRepairPlan | null,
  jdkDistributions: [] as JdkDistribution[],
  update: null as UpdateCheckResult | null,
  updateError: "",
  updateDownloaded: false,
  agentTraces: null as AgentTraceReport | null,
  cleanupArchitecture: null as CleanupArchitecture | null,
  cleanupReport: null as CleanupScanReport | null,
  cleanupSelection: new Set<string>(),
  cleanupPlan: null as CleanupPlan | null,
  cleanupResult: null as CleanupResult | null,
  maintenanceOverview: null as MaintenanceOverview | null,
  desktopUsage: null as FolderUsageReport | null,
  downloadsUsage: null as FolderUsageReport | null,
  largeFiles: [] as LargeFileItem[],
  archivePlan: [] as ArchivePlanItem[],
  movePlan: null as MovePlan | null,
  moveResult: null as MoveResult | null,
  rollbackRecords: [] as RollbackRecord[],
  partitionLayout: null as PartitionLayoutReport | null,
  expansionPlan: null as ExpansionPlan | null,
  expansionResult: null as ExpansionResult | null,
  duplicateGroups: [] as DuplicateGroup[],
  appUsage: null as AppUsageReport | null,
  safeMode: false,
  fatalError: "",
  safeModeNoticeCollapsed: false,
};

const paginationState = new Map<string, number>();
const PAGE_SIZE = 5;

function paginate<T>(key: string, items: T[], render: (item: T) => string, pageSize = PAGE_SIZE) {
  if (items.length <= pageSize) return items.map(render).join("");
  const pages = Math.ceil(items.length / pageSize);
  const page = Math.min(Math.max(1, paginationState.get(key) || 1), pages);
  paginationState.set(key, page);
  const content = items.slice((page - 1) * pageSize, page * pageSize).map(render).join("");
  return `${content}<nav class="pagination" aria-label="分页"><button data-page-key="${escapeHtml(key)}" data-page="${page - 1}" ${page <= 1 ? "disabled" : ""}>上一页</button><span>${page} / ${pages} · 共 ${items.length} 项</span><button data-page-key="${escapeHtml(key)}" data-page="${page + 1}" ${page >= pages ? "disabled" : ""}>下一页</button></nav>`;
}

function paginationControls(key: string, count: number, pageSize = PAGE_SIZE) {
  if (count <= pageSize) return "";
  const pages = Math.ceil(count / pageSize);
  const page = Math.min(Math.max(1, paginationState.get(key) || 1), pages);
  paginationState.set(key, page);
  return `<nav class="pagination" aria-label="分页"><button data-page-key="${escapeHtml(key)}" data-page="${page - 1}" ${page <= 1 ? "disabled" : ""}>上一页</button><span>${page} / ${pages} · 共 ${count} 项</span><button data-page-key="${escapeHtml(key)}" data-page="${page + 1}" ${page >= pages ? "disabled" : ""}>下一页</button></nav>`;
}

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
    ? paginate("path-warnings", state.env.pathWarnings, (item) => {
          const kind = item.startsWith("托管 PATH") ? "pending" : item.startsWith("重复 PATH") ? "duplicate" : "invalid";
          return `<div class="warning ${kind}">${escapeHtml(item)}</div>`;
        })
    : `<div class="empty">当前进程 PATH 没有发现重复或失效条目</div>`;
}

function renderRuntimes() {
  setText("metric-runtimes", state.runtimes.length);
  renderEffectiveRuntimes();
  const element = document.querySelector<HTMLElement>("#runtime-list");
  if (!element) return;
  element.innerHTML = state.runtimes.length
    ? paginate("runtime-list", state.runtimes,
          (runtime) => `
            <article class="runtime">
              <div><strong>${escapeHtml(runtime.kind)}</strong><span>${escapeHtml(runtime.version)}</span></div>
              <small>${escapeHtml(runtime.source)} · ${escapeHtml(runtime.executable)}</small>
              ${runtimeSafeActions(runtime)}
            </article>
          `)
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
    <section class="notice-panel">
      <h3>JDK 候选</h3>
      <div class="runtime-list">
        ${report.candidates.length ? report.candidates.map((candidate) => {
          const javaHome = candidate.executable.replace(/\\bin\\java\.exe$/i, "");
          const checks = state.externalJdkChecks[javaHome] || [];
          return `
          <article class="runtime">
            <div><strong>${escapeHtml(candidate.version.split("\n")[0] || "Java")}</strong><span>${escapeHtml(candidate.source)}</span></div>
            <small>${escapeHtml(candidate.executable)}</small>
            ${checks.length ? `<div class="runtime-list compact-list">${renderValidationChecks(checks)}</div>` : ""}
            <div class="row-actions">
              <button data-action="open-analysis-path" data-path="${escapeHtml(candidate.executable)}">${icon(FolderSearch)}<span>打开目录</span></button>
              <button data-action="copy-text" data-copy="${escapeHtml(candidate.executable)}">${icon(Clipboard)}<span>复制路径</span></button>
              <button data-action="copy-text" data-copy="${escapeHtml(javaHome)}">${icon(Clipboard)}<span>复制 JAVA_HOME 候选</span></button>
              <button data-action="verify-external-jdk" data-jdk-path="${escapeHtml(javaHome)}">${icon(Shield)}<span>验证此 JDK</span></button>
              <button data-action="set-java-home-candidate" data-jdk-path="${escapeHtml(javaHome)}">${icon(Route)}<span>设为用户级 JAVA_HOME</span></button>
            </div>
          </article>
        `;
        }).join("") : `<div class="empty">没有发现 JDK 候选</div>`}
      </div>
      <ul>
        <li>外部、IDE 内置、Scoop、Chocolatey、mise、asdf JDK 不会被 DevEnv Manager 卸载、删除或移入回收站。</li>
        <li>如需接管，请先复制 JAVA_HOME 候选并在环境配置里预览用户级修复计划；不要自动改系统级 PATH。</li>
      </ul>
    </section>
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
    ? paginate("managed-jdks", jdks,
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
          `)
    : `<div class="empty">还没有安装受管 JDK</div>`;
}

function renderManagedNodes() {
  const element = document.querySelector<HTMLElement>("#managed-nodes");
  if (!element) return;
  const nodes = state.config?.installed.nodes || [];
  const current = state.config?.installed.current.node;
  element.innerHTML = nodes.length
    ? paginate("managed-nodes", nodes,
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
          `)
    : `<div class="empty">还没有安装受管 Node.js</div>`;
}

function renderManagedPythons() {
  const element = document.querySelector<HTMLElement>("#managed-pythons");
  if (!element) return;
  const pythons = state.config?.installed.pythons || [];
  const current = state.config?.installed.current.python;
  element.innerHTML = pythons.length
    ? paginate("managed-pythons", pythons,
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
          `)
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
    ? paginate("managed-build-tools", items,
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
          `)
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
    ? paginate("managed-gos", gos,
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
          `)
    : `<div class="empty">还没有安装受管 Go</div>`;
}

function renderPorts() {
  const visible = sortedPorts(filteredPorts(state.ports));
  setText("metric-ports", visible.length);
  renderPortSummary();
  const body = document.querySelector<HTMLElement>("#ports-body");
  if (!body) return;
  const pageSize = 10;
  const pages = Math.max(1, Math.ceil(visible.length / pageSize));
  const page = Math.min(Math.max(1, paginationState.get("ports") || 1), pages);
  paginationState.set("ports", page);
  body.innerHTML = visible
    .slice((page - 1) * pageSize, page * pageSize)
    .map(
      (record) => {
        const conflict = record.conflictCount || record.conflictEvidence?.length || 0;
        const selected = state.selectedPort?.pid === record.pid && state.selectedPort.localPort === record.localPort;
        return `
        <tr class="${selected ? "selected" : ""}">
          <td><strong>${record.localPort}</strong><span class="port-hint">${escapeHtml(record.protocol)} · ${escapeHtml(record.localAddress)}</span></td>
          <td>${escapeHtml(record.state)}</td>
          <td><strong>${escapeHtml(record.identity || record.commonUsage || "Unknown")}</strong>${conflict ? `<button class="conflict-badge" data-action="port-details" data-pid="${record.pid}" data-port="${record.localPort}" title="${escapeHtml((record.conflictEvidence || []).join("；") || "存在冲突证据")}">${conflict}冲突</button>` : ""}</td>
          <td>${escapeHtml(record.processName || "未读取")}</td>
          <td>${record.pid}</td>
          <td><span class="pill ${record.confidence >= 70 ? "ok" : record.confidence >= 40 ? "warn" : "muted"}">${confidenceLabel(record.confidence)}</span><small>${record.evidenceCount || record.evidence?.length || 0}证据</small></td>
          <td><span class="risk-chip risk-${portRiskClass(record.riskLevel || record.risk)}">${escapeHtml(record.riskLevel || record.risk)}</span></td>
          <td><button class="icon-action" data-action="port-details" data-pid="${record.pid}" data-port="${record.localPort}" title="详情">${icon(Search)}</button></td>
        </tr>
      `;
      },
    )
    .join("");
  const pager = document.querySelector<HTMLElement>("#ports-pagination");
  if (pager) pager.innerHTML = paginationControls("ports", visible.length, pageSize);
  updateSortHeaders();
  renderPortDetails();
  renderPortHistory();
}

function renderPortSummary() {
  const element = document.querySelector<HTMLElement>("#port-summary");
  if (!element) return;
  const records = state.ports;
  const count = (predicate: (record: PortRecord) => boolean) => records.filter(predicate).length;
  const items = [
    ["listening", "监听端口", count((record) => record.state.toLowerCase() === "listening")],
    ["development", "开发服务", count((record) => ["development", "frontend", "backend", "python"].includes(portCategory(record)))],
    ["database", "数据库/中间件", count((record) => ["database", "middleware"].includes(portCategory(record)))],
    ["desktop", "桌面应用", count((record) => portCategory(record) === "desktop")],
    ["sensitive", "高风险/系统", count((record) => ["high", "critical", "blocked", "阻止", "高风险"].includes((record.riskLevel || record.risk).toLowerCase()))],
    ["low-confidence", "低置信度", count((record) => record.confidence < 40 || (record.conflictCount || 0) > 0)],
  ];
  element.innerHTML = items
    .map(([filter, label, value]) => `<button class="port-summary-card" data-port-filter="${filter}"><strong>${value}</strong><span>${label}</span></button>`)
    .join("");
}

function renderPortDetails() {
  const element = document.querySelector<HTMLElement>("#port-detail");
  if (!element) return;
  const record = state.selectedPort;
  if (!record) {
    element.innerHTML = `<div class="empty">点击端口行查看详情</div>`;
    return;
  }
  const history = state.portHistory
    .filter((item) => item.port === record.localPort)
    .slice(0, 6);
  const isHttpLike = /\b(http|web|vite|spring|tomcat|next|nuxt|flask|django|fastapi|uvicorn)\b/i.test(`${record.identity} ${record.commonUsage}`);
  const isDatabase = portCategory(record) === "database";
  element.innerHTML = `
    <div class="panel-title compact-title">${icon(Network)}<h2>${record.localPort} · ${escapeHtml(record.identity || record.commonUsage || "Unknown")}</h2></div>
    <p class="port-explanation">${escapeHtml(record.recommendation || record.explanation)}</p>
    <div class="kv-list port-detail-kv">
      <div><span>协议/地址</span><strong>${escapeHtml(record.protocol)} · ${escapeHtml(record.localAddress)} · ${escapeHtml(record.state)}</strong></div>
      <div><span>进程</span><strong>${escapeHtml(record.processName || "未读取")} / PID ${record.pid}</strong></div>
      <div><span>进程路径</span><strong>${escapeHtml(record.processPath || "未读取")}</strong></div>
      <div><span>启动命令</span><strong>${escapeHtml(record.commandLine || "未读取")}</strong></div>
      <div><span>父进程</span><strong>${escapeHtml(record.parentProcessName || "未读取")}${record.parentPid ? ` / PID ${record.parentPid}` : ""}</strong></div>
      <div><span>Windows 服务</span><strong>${escapeHtml(record.serviceNames.join("、") || "未识别")}</strong></div>
      <div><span>置信度</span><strong>${confidenceLabel(record.confidence)} · ${record.evidenceCount || record.evidence?.length || 0} 条证据</strong></div>
      <div><span>风险</span><strong>${escapeHtml(record.riskLevel || record.risk)}</strong></div>
    </div>
    <details open><summary>识别证据</summary><ul>${(record.evidence || []).map((item) => `<li>${escapeHtml(item)}</li>`).join("") || "<li>证据不足，不能只凭端口号判断服务类型。</li>"}</ul></details>
    <details ${record.conflictEvidence?.length ? "open" : ""}><summary>冲突证据</summary><ul>${(record.conflictEvidence || []).map((item) => `<li>${escapeHtml(item)}</li>`).join("") || "<li>未发现明显冲突证据。</li>"}</ul></details>
    <details><summary>最近 7 天历史</summary>${history.length ? `<ul>${history.map((item) => `<li>${escapeHtml(item.processName)} · ${item.observations} 次 · ${new Date(item.lastSeen * 1000).toLocaleString("zh-CN")}</li>`).join("")}</ul>` : "<p>还没有该端口历史。</p>"}</details>
    <div class="toolbar compact port-detail-actions">
      <button data-action="copy-text" data-copy="${escapeHtml(record.processPath)}">${icon(Clipboard)}<span>复制路径</span></button>
      ${record.processPath ? `<button data-action="open-process-location" data-pid="${record.pid}">${icon(FolderSearch)}<span>打开位置</span></button>` : ""}
      <button data-action="copy-text" data-copy="${escapeHtml(portDiagnosticSummary(record))}">${icon(Clipboard)}<span>复制摘要</span></button>
      ${isHttpLike ? `<button data-action="copy-text" data-copy="curl -I http://127.0.0.1:${record.localPort}">${icon(Clipboard)}<span>复制 curl</span></button>` : ""}
      ${isDatabase ? `<button data-action="copy-text" data-copy="${escapeHtml(databaseCommandHint(record))}">${icon(Database)}<span>复制连接命令</span></button>` : ""}
      ${canShowKillPortAction(record) ? `<button class="danger-button" data-action="kill-port" data-pid="${record.pid}">${icon(Trash2)}<span>安全结束</span></button>` : `<span class="small-note">系统关键或高风险进程不提供结束入口</span>`}
    </div>
  `;
}

function renderPortHistory() {
  const element = document.querySelector<HTMLElement>("#port-history");
  if (!element) return;
  element.innerHTML = state.portHistory.length
    ? paginate("port-history", state.portHistory,
          (item) => `
            <article class="runtime">
              <div><strong>${item.port} · ${escapeHtml(item.processName)}</strong><span>${item.observations} 次</span></div>
              <small>最近记录：${new Date(item.lastSeen * 1000).toLocaleString("zh-CN")}</small>
            </article>
          `)
    : `<div class="empty">还没有端口历史</div>`;
}

function runtimeSafeActions(runtime: RuntimeInfo) {
  const external = !runtime.source.toLowerCase().includes("devenv");
  if (!external) return "";
  const canOpenApps = ["System", "Microsoft Store", "Scoop", "Chocolatey", "PATH"].includes(runtime.source);
  return `<div class="row-actions">
    <button data-action="open-analysis-path" data-path="${escapeHtml(runtime.executable)}">${icon(FolderSearch)}<span>打开位置</span></button>
    <button data-action="copy-text" data-copy="${escapeHtml(runtime.executable)}">${icon(Clipboard)}<span>复制路径</span></button>
    ${canOpenApps ? `<button data-action="open-apps-features">${icon(FolderOpen)}<span>系统卸载入口</span></button>` : ""}
  </div>`;
}

function renderHealth() {
  const element = document.querySelector<HTMLElement>("#env-health");
  if (!element) return;
  element.innerHTML = state.health.length
    ? paginate("env-health", state.health,
          (item) => `
            <article class="runtime health-item ${item.status === "正常" ? "ok" : "warn"}">
              <div><strong>${escapeHtml(item.name)}</strong><span>${escapeHtml(item.status)}</span></div>
              <small>${escapeHtml(item.detail)}</small>
            </article>
          `)
    : `<div class="empty">还没有环境健康检查结果</div>`;
}

function renderProfiles() {
  const element = document.querySelector<HTMLElement>("#profile-list");
  if (!element) return;
  element.innerHTML = state.profiles.length
    ? paginate("profiles", state.profiles, (profile) => {
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
    <div class="kv-list toolchain-kv">
      <div><span>PATH 首个 python</span><strong>${escapeHtml(analysis.firstPythonOnPath || "未发现")}</strong></div>
      <div><span>PATH 首个 pip</span><strong>${escapeHtml(analysis.firstPipOnPath || "未发现")}</strong></div>
      <div><span>python -m pip</span><strong>${analysis.pythonMPipAvailable ? "可用" : "不可用"}</strong></div>
      <div><span>Python Launcher</span><strong>${escapeHtml(analysis.launcherPath || "未发现")}</strong></div>
      <div><span>用户 PATH</span><strong>${analysis.userPathEntryCount} 项 · ${analysis.currentTerminalMatchesUserPath ? "当前进程已同步" : "当前进程仍是旧 PATH"}</strong></div>
      <div><span>Store 执行别名</span><strong>${analysis.storeAliasRisk ? "可能抢占" : "未发现抢占"}</strong></div>
      <div><span>受管 Python</span><strong>${analysis.managedPythonAvailable ? "已安装，可生成切换/修复计划" : "未安装，请先安装受管 Python"}</strong></div>
    </div>
    <div class="chip-row">${analysis.risks.map((item) => `<span>${escapeHtml(item)}</span>`).join("")}</div>
    ${analysis.repairBlockers.length ? `<section class="notice-panel"><h3>阻断修复计划的原因</h3><ul>${analysis.repairBlockers.map((item) => `<li>${escapeHtml(item)}</li>`).join("")}</ul></section>` : ""}
    <section class="notice-panel"><h3>下一步安全操作</h3><ul>${analysis.recoveryActions.map((item) => `<li>${escapeHtml(item)}</li>`).join("")}</ul></section>
    <div class="toolbar compact">
      <button data-action="copy-text" data-copy="${escapeHtml(analysis.pipRepairCommand)}">${icon(Clipboard)}<span>复制 pip 修复命令</span></button>
      <button data-action="copy-text" data-copy="${escapeHtml(analysis.aliasSettingsCommand)}">${icon(Clipboard)}<span>复制别名设置命令</span></button>
      <button data-action="open-python-alias-settings">${icon(FolderOpen)}<span>打开执行别名设置</span></button>
      <button data-action="export-python-diagnostic">${icon(FileText)}<span>导出只读诊断</span></button>
      <button data-action="copy-text" data-copy="${escapeHtml(analysis.diagnosticReport)}">${icon(Clipboard)}<span>复制诊断报告</span></button>
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

function renderPythonRepairPlan() {
  const element = document.querySelector<HTMLElement>("#python-repair-preview");
  const plan = state.pythonRepairPlan;
  if (!element) return;
  if (!plan) {
    element.innerHTML = `<div class="empty">先分析，再预览；不会卸载其他 Python 或自动关闭 Store 别名</div>`;
    return;
  }
  element.innerHTML = `<article class="repair-plan-card"><div><strong>${escapeHtml(plan.pythonPath)}</strong><span>一次性计划 · 备份 ${escapeHtml(plan.backupName)}</span></div>
    <h4>将执行</h4><ol>${plan.actions.map((item) => `<li>${escapeHtml(item)}</li>`).join("")}</ol>
    ${plan.pathAdded.length ? `<h4>PATH 新增</h4><code>${plan.pathAdded.map(escapeHtml).join("<br>")}</code>` : ""}
    ${plan.commands.length ? `<h4>命令</h4><pre class="command-output compact-output">${escapeHtml(plan.commands.join("\n"))}</pre>` : ""}
    <ul>${plan.warnings.map((item) => `<li>${escapeHtml(item)}</li>`).join("")}</ul>
    <button id="apply-python-repair" class="primary">二次确认并执行</button></article>`;
}

function renderValidationChecks(checks: ValidationCheck[]) {
  return checks.map((check) => `
    <article class="runtime ${check.success ? "" : check.required ? "warn" : "notice"}">
      <div><strong>${escapeHtml(check.title)}</strong><span>${check.success ? "通过" : check.required ? "必需项失败" : "可选项提示"}</span></div>
      <small>${escapeHtml(check.stage)} · ${escapeHtml(check.detail || "无输出")}</small>
    </article>
  `).join("");
}

function renderPythonIntegrity() {
  const element = document.querySelector<HTMLElement>("#python-integrity-result");
  if (!element) return;
  const report = state.pythonIntegrity;
  element.innerHTML = report
    ? `<article class="runtime">
        <div><strong>${escapeHtml(report.status)}</strong>${riskBadge(report.fullyUsable ? "info" : "high")}</div>
        <small>${escapeHtml(report.pythonPath)} · ${report.managed ? "受管 Python" : "非受管 Python"}</small>
      </article>
      ${renderValidationChecks(report.checks)}
      ${report.risks.length ? `<ul>${report.risks.map((item) => `<li>${escapeHtml(item)}</li>`).join("")}</ul>` : ""}
      ${report.suggestions.length ? `<ul>${report.suggestions.map((item) => `<li>${escapeHtml(item)}</li>`).join("")}</ul>` : ""}
      <button id="create-managed-python-pip-plan" ${report.managed ? "" : "disabled"}>生成受管 Python pip 修复计划</button>`
    : `<div class="empty">点击“完整性检查”验证 pip、venv、ssl、sqlite3、ctypes 和 tkinter。</div>`;
}

function renderRuntimeStrongVerification() {
  const element = document.querySelector<HTMLElement>("#runtime-strong-result");
  if (!element) return;
  const report = state.runtimeStrong;
  element.innerHTML = report
    ? `<div class="scan-only-banner">${icon(Shield)}<span>${report.summary.map(escapeHtml).join(" ")}</span></div>
      ${paginate("runtime-strong", report.items, (item) => `
        <article class="runtime">
          <div><strong>${escapeHtml(item.kind)} ${escapeHtml(item.version)}</strong><span>${escapeHtml(item.status)}</span></div>
          <small>${escapeHtml(item.path)} · current=${item.current ? "是" : "否"} · 环境生效=${item.environmentEffective ? "是" : "否"}${item.failureStage ? ` · 失败阶段=${escapeHtml(item.failureStage)}` : ""}</small>
          <div class="runtime-list compact-list">${renderValidationChecks(item.checks)}</div>
        </article>
      `)}`
    : `<div class="empty">检查 JDK/Python/Node/Maven/Gradle/Go 的登记、组件、current 指针和环境生效状态。</div>`;
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
  const chsrc = document.querySelector<HTMLElement>("#chsrc-status");
  const chsrcRecovery = document.querySelector<HTMLElement>("#chsrc-recovery");
  const report = state.platforms;
  if (!go || !rust || !dotnet || !mirrors || !report) return;
  if (chsrc) {
    chsrc.textContent = report.chsrc.installed ? `已安装 · ${report.chsrc.version}` : "未安装";
    chsrc.className = `risk-chip ${report.chsrc.installed ? "risk-low" : "risk-medium"}`;
  }
  if (chsrcRecovery) {
    const recovery = report.chsrcRecovery;
    chsrcRecovery.innerHTML = `
      <h3>${recovery.missing ? "chsrc 缺失闭环" : "chsrc 安全边界"}</h3>
      <ul>${recovery.explanation.map((item) => `<li>${escapeHtml(item)}</li>`).join("")}</ul>
      ${recovery.missing ? `<div class="toolbar compact">
        <button data-action="copy-text" data-copy="${escapeHtml(recovery.scoopCommand)}">${icon(Clipboard)}<span>复制 Scoop 命令</span></button>
        <button data-action="copy-text" data-copy="${escapeHtml(recovery.wingetCommand)}">${icon(Clipboard)}<span>复制 WinGet 命令</span></button>
        <button data-action="copy-text" data-copy="${escapeHtml(recovery.officialUrl)}">${icon(Clipboard)}<span>复制官方项目</span></button>
        <button data-action="refresh-platforms">${icon(RefreshCw)}<span>重新检测</span></button>
      </div>` : ""}
      <div class="chip-row">${recovery.fallbackFeatures.map((item) => `<span>${escapeHtml(item)}</span>`).join("")}</div>
    `;
  }

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
    <div class="chip-row">${paginate("rust-toolchains", report.rust.installedToolchains, (item) => `<span>${escapeHtml(item)}</span>`) || ""}</div>
  `;
  dotnet.innerHTML = `
    <div class="tool-state-grid">${renderToolStates([report.dotnet.dotnet])}</div>
    <div class="platform-columns">
      <div><h3>SDK</h3><div class="runtime-list">${paginate("dotnet-sdks", report.dotnet.sdks, (item) => `<div class="line-item">${escapeHtml(item)}</div>`) || "未发现 SDK"}</div></div>
      <div><h3>Runtime</h3><div class="runtime-list">${paginate("dotnet-runtimes", report.dotnet.runtimes, (item) => `<div class="line-item">${escapeHtml(item)}</div>`) || "未发现 Runtime"}</div></div>
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

async function checkUpdates() {
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
    const category = portCategory(record);
    const riskClass = portRiskClass(record.riskLevel || record.risk);
    const conflictCount = record.conflictCount || record.conflictEvidence?.length || 0;
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
      record.identity,
      record.riskLevel,
      record.recommendation,
      record.evidence.join(" "),
      record.conflictEvidence.join(" "),
      category,
      hint,
    ]
      .join(" ")
      .toLowerCase();
    const queryMatch = terms.length === 0 || terms.every((term) => text.includes(term));
    const quickMatch = (() => {
      switch (portState.quickFilter) {
        case "all":
          return true;
        case "listening":
          return record.state.toLowerCase() === "listening";
        case "development":
          return ["development", "frontend", "backend", "python"].includes(category);
        case "frontend":
        case "backend":
        case "python":
        case "database":
        case "middleware":
        case "desktop":
          return category === portState.quickFilter;
        case "sensitive":
          return ["high", "critical"].includes(riskClass) || record.risk !== "普通";
        case "low-confidence":
          return record.confidence < 40 || conflictCount > 0;
        default:
          return commonPorts.some(
            (item) =>
              item.key === portState.quickFilter &&
              (item.ports.includes(record.localPort) ||
                item.keywords.some((keyword) => record.processName.toLowerCase().includes(keyword))),
          );
      }
    })();
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

function confidenceLabel(value: number) {
  if (value >= 70) return `高 ${value}%`;
  if (value >= 40) return `中 ${value}%`;
  if (value > 0) return `低 ${value}%`;
  return "未知";
}

function portRiskClass(value: string) {
  const risk = (value || "").toLowerCase();
  if (risk.includes("critical") || risk.includes("blocked") || risk.includes("阻止") || risk.includes("严重")) return "critical";
  if (risk.includes("high") || risk.includes("高")) return "high";
  if (risk.includes("medium") || risk.includes("warn") || risk.includes("谨慎") || risk.includes("中")) return "medium";
  if (risk.includes("low") || risk.includes("普通") || risk.includes("低")) return "low";
  return "medium";
}

function portCategory(record: PortRecord) {
  const text = [
    record.identity,
    record.commonUsage,
    record.processName,
    record.processPath,
    record.commandLine,
    record.parentProcessName,
    record.serviceNames.join(" "),
    record.evidence.join(" "),
  ]
    .join(" ")
    .toLowerCase();
  if (/\b(mysql|mariadb|postgres|postgresql|psql|redis|mongodb|mongod|mongo|sqlite|sql server|mssql|oracle)\b/.test(text)) return "database";
  if (/\b(rabbitmq|kafka|zookeeper|nacos|consul|etcd|minio|elasticsearch|opensearch|memcached)\b/.test(text)) return "middleware";
  if (/\b(electron|tauri|wails|cef|webview|qt|wpf|winforms|javaw)\b/.test(text)) return "desktop";
  if (/\b(vite|webpack|next|nuxt|react|vue|angular|svelte|astro|storybook|frontend)\b/.test(text)) return "frontend";
  if (/\b(python|py\.exe|python\.exe|uvicorn|gunicorn|flask|django|fastapi|jupyter|streamlit|gradio)\b/.test(text)) return "python";
  if (/\b(java|node|deno|bun|spring|tomcat|jetty|express|nestjs|koa|gin|go\.exe|cargo|backend|api)\b/.test(text)) return "backend";
  if (record.localPort >= 3000 && record.localPort <= 9999) return "development";
  return "unknown";
}

function portDiagnosticSummary(record: PortRecord) {
  const parts = [
    `端口 ${record.localPort}/${record.protocol} ${record.state}`,
    `进程 ${record.processName || "未读取"} PID ${record.pid}`,
    `识别 ${record.identity || record.commonUsage || "Unknown"}，置信度 ${confidenceLabel(record.confidence)}`,
    `风险 ${record.riskLevel || record.risk}`,
  ];
  if (record.processPath) parts.push(`路径 ${record.processPath}`);
  if (record.commandLine) parts.push(`命令 ${record.commandLine}`);
  if (record.evidence.length) parts.push(`证据 ${record.evidence.join("；")}`);
  if (record.conflictEvidence.length) parts.push(`冲突 ${record.conflictEvidence.join("；")}`);
  return parts.join("\n");
}

function databaseCommandHint(record: PortRecord) {
  const text = `${record.identity} ${record.commonUsage} ${record.processName} ${record.commandLine}`.toLowerCase();
  if (text.includes("redis")) return `redis-cli -h 127.0.0.1 -p ${record.localPort}`;
  if (text.includes("postgres") || text.includes("psql")) return `psql -h 127.0.0.1 -p ${record.localPort} -U postgres`;
  if (text.includes("mongo")) return `mongosh "mongodb://127.0.0.1:${record.localPort}"`;
  if (text.includes("mysql") || text.includes("mariadb")) return `mysql -h 127.0.0.1 -P ${record.localPort} -u root -p`;
  if (text.includes("mssql") || text.includes("sql server")) return `sqlcmd -S 127.0.0.1,${record.localPort} -E`;
  return `连接 127.0.0.1:${record.localPort}`;
}

function portHint(record: PortRecord) {
  const exact = commonPorts.filter((item) => item.ports.includes(record.localPort)).map((item) => item.label);
  if (record.commonUsage && record.commonUsage !== "未识别的本地服务") exact.push(record.commonUsage);
  if (record.identity) exact.push(record.identity);
  if ((record.riskLevel || record.risk) !== "普通") exact.push(record.riskLevel || record.risk);
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
    <div class="chip-row">${paginate("project-signals", analysis.detectedFiles, (item) => `<span>${escapeHtml(item)}</span>`)}</div>
    <div class="grid two compact-grid">
      <section>
        <h3>推荐环境</h3>
        <div class="runtime-list">
          ${paginate("project-runtimes", analysis.recommendedRuntime,
              (item) => `
                <article class="runtime">
                  <div><strong>${escapeHtml(item.name)}</strong><span>${escapeHtml(item.status)}</span></div>
                  <small>${escapeHtml(item.requirement)}</small>
                </article>
              `) || `<div class="empty">没有特殊版本要求</div>`}
        </div>
      </section>
      <section>
        <h3>建议操作</h3>
        <div class="runtime-list">
          ${paginate("project-actions", analysis.actions,
              (item) => `
                <article class="runtime project-action-item">
                  <div><strong>${escapeHtml(item.title)}</strong><span>${escapeHtml(item.command)}</span></div>
                  <small>${escapeHtml(item.description)}</small>
                  <button data-action="project-run" data-project-action="${escapeHtml(item.id)}">${item.id === "copy_commands" ? icon(Clipboard) : icon(Play)}<span>${item.id === "copy_commands" ? "复制" : "运行"}</span></button>
                </article>
              `)}
        </div>
      </section>
    </div>
    ${analysis.warnings.length ? `<ul>${analysis.warnings.map((item) => `<li>${escapeHtml(item)}</li>`).join("")}</ul>` : ""}
  `;
}

function renderIdeaProject() {
  const element = document.querySelector<HTMLElement>("#idea-project-result");
  if (!element) return;
  const report = state.ideaProject;
  element.innerHTML = report
    ? `<article class="runtime">
        <div><strong>${report.detected ? "已读取 IDEA 配置" : "未发现 IDEA 配置"}</strong><span>${escapeHtml(report.jdkMatch)}</span></div>
        <small>${escapeHtml(report.root)}</small>
      </article>
      <div class="kv-list">
        <div><span>Project SDK</span><strong>${escapeHtml(report.projectSdk || "未显式配置")}</strong></div>
        <div><span>Language Level</span><strong>${escapeHtml(report.languageLevel || "未读取")}</strong></div>
        <div><span>Compiler target</span><strong>${escapeHtml(report.compilerTarget || "未读取")}</strong></div>
        <div><span>Gradle JVM</span><strong>${escapeHtml(report.gradleJvm || "未显式配置")}</strong></div>
        <div><span>Maven importer JDK</span><strong>${escapeHtml(report.mavenImporterJdk || "未显式配置")}</strong></div>
        <div><span>模块数量</span><strong>${report.moduleCount}</strong></div>
      </div>
      <div class="chip-row">${report.readFiles.map((item) => `<span>${escapeHtml(item)}</span>`).join("")}</div>
      ${report.moduleSdks.length ? `<div class="chip-row">${report.moduleSdks.map((item) => `<span>Module SDK: ${escapeHtml(item)}</span>`).join("")}</div>` : ""}
      ${report.warnings.length ? `<ul>${report.warnings.map((item) => `<li>${escapeHtml(item)}</li>`).join("")}</ul>` : ""}`
    : `<div class="empty">选择项目文件夹后，可只读分析 .idea/misc.xml、compiler.xml 和 *.iml。</div>`;
}

function renderJavaConsumer() {
  const element = document.querySelector<HTMLElement>("#java-consumer-result");
  if (!element) return;
  const report = state.javaConsumer;
  element.innerHTML = report
    ? `<article class="runtime ${report.usable ? "" : "warn"}">
        <div><strong>${escapeHtml(report.consumer)} Java 环境</strong><span>${report.usable ? "可读取" : "需要关注"}</span></div>
        <small>${escapeHtml(report.root)}</small>
      </article>
      <div class="kv-list">
        <div><span>启动入口/项目文件</span><strong>${report.startupExists ? "存在" : "未发现"}</strong></div>
        <div><span>JAVA_HOME raw</span><strong>${escapeHtml(report.javaHomeRaw || "未设置")}</strong></div>
        <div><span>JAVA_HOME expanded</span><strong>${escapeHtml(report.javaHomeExpanded || "未设置")}</strong></div>
        <div><span>java.exe</span><strong>${report.javaExists ? "存在" : "缺失"}</strong></div>
        <div><span>javac.exe</span><strong>${report.javacExists ? "存在" : "缺失"}</strong></div>
        <div><span>PATH 首个 java</span><strong>${escapeHtml(report.pathJava || "未发现")}</strong></div>
      </div>
      <ul>${report.explanation.map((item) => `<li>${escapeHtml(item)}</li>`).join("")}</ul>`
    : `<div class="empty">Nacos/Nexus/Maven/Gradle/Spring Boot 等 Java 消费者验证会读取最新用户环境，不修改项目。</div>`;
}

function renderProjectPortConfigs() {
  const element = document.querySelector<HTMLElement>("#project-port-configs");
  if (!element) return;
  element.innerHTML = state.projectPorts.length
    ? paginate("project-ports", state.projectPorts, (config) => `
        <article class="runtime project-port-item">
          <div><strong>${escapeHtml(config.description)}</strong><span>当前 ${config.currentPort}</span></div>
          <small>${escapeHtml(config.file)}${config.line ? ` · 第 ${config.line} 行` : " · 将创建配置"}</small>
          <div class="row-actions port-config-actions">
            <input type="number" min="1024" max="65535" value="${config.currentPort + 1}" data-port-config-input="${escapeHtml(config.id)}" aria-label="新端口" />
            <button data-action="update-project-port" data-config-id="${escapeHtml(config.id)}">${icon(RefreshCw)}<span>备份并修改</span></button>
          </div>
        </article>
      `)
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
  const [snapshot, config, envSnapshot, profiles, jdkDistributions, cleanupArchitecture, environmentBackups, safetyDisclaimer, featureRisks] = await Promise.all([
    invoke<AppSnapshot>("app_snapshot"),
    invoke<ConfigView>("load_config"),
    invoke<EnvSnapshot>("env_snapshot"),
    invoke<ConfigProfile[]>("list_config_profiles"),
    invoke<JdkDistribution[]>("jdk_distributions"),
    invoke<CleanupArchitecture>("storage_cleanup_architecture"),
    invoke<EnvironmentBackupInfo[]>("list_environment_backups"),
    invoke<string>("safety_disclaimer"),
    invoke<FeatureRiskInfo[]>("feature_risk_registry"),
  ]);

  state.snapshot = snapshot;
  state.config = config;
  state.env = envSnapshot;
  state.profiles = profiles;
  state.jdkDistributions = jdkDistributions;
  state.cleanupArchitecture = cleanupArchitecture;
  state.environmentBackups = environmentBackups;
  state.safetyDisclaimer = safetyDisclaimer;
  state.featureRisks = featureRisks;
  renderSnapshot();
  renderEnv();
  renderEnvironmentPreview();
  renderEnvironmentBackups();
  renderSafetyDisclaimer();
  renderEnvReliability();
  renderEnvRepairPlan();
  renderEnvBackupRecords();
  renderHealth();
  renderProfiles();
  renderProfileImportPreview();
  renderDoctor();
  renderPythonAnalysis();
  renderPythonIntegrity();
  renderRuntimeStrongVerification();
  renderRuntimes();
  renderJdkDistributions();
  renderJavaEnvironment();
  renderAgentTraces();
  renderUpdate();
  renderSafetyGate();
  renderFatalError();
  renderMaintenanceOverview();
  renderMaintenanceScan();
  renderCleanupPlan();
  renderCleanupResult();
  renderProjectConfigPreview();
  renderIdeaProject();
  renderJavaConsumer();
  renderFolderUsage("#desktop-usage", state.desktopUsage, "desktop-usage");
  renderFolderUsage("#downloads-usage", state.downloadsUsage, "downloads-usage");
  renderLargeFiles();
  renderDuplicates();
  renderAppUsage();
  renderPorts();
  renderViewGuide(undefined, state.featureRisks, escapeHtml);
  clearFeatureHelp();
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
  if (!(await askForConfirmation(`将结束 ${label} 及其子进程。确定继续吗？`))) return;
  try {
    const planId = `pid-${pid}-force-false-allow-false`;
    const fingerprint = await processActionFingerprint("kill_process", planId, "high");
    const token = await createBackendConfirmation("kill_process", planId, "high", fingerprint, false);
    let result = await invoke<KillResult>("kill_process", { pid, force: false, allowCaution: false, confirmationToken: token.token });
    if (result.needsForce) {
      const force = await askForConfirmation(`${result.message}\n\n是否改为强制结束？`);
      if (!force) {
        showToast("已取消强制结束");
        return;
      }
      if (!(await askForConfirmation("强制结束是极高风险操作。第一次确认：我已保存相关工作。"))) return;
      if (!(await askForConfirmation("第二次确认：我理解这可能导致数据未保存或服务中断。"))) return;
      const forcePlanId = `pid-${pid}-force-true-allow-false`;
      const forceFingerprint = await processActionFingerprint("kill_process", forcePlanId, "critical");
      const forceToken = await createBackendConfirmation("kill_process", forcePlanId, "critical", forceFingerprint, true);
      result = await invoke<KillResult>("kill_process", { pid, force: true, allowCaution: false, confirmationToken: forceToken.token });
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
    await runOperation(async () => {
      const token = await riskOperationToken("cleanup_path_entries", "cleanup-path-entries", "medium", false, "environment-backup");
      return invoke<OperationResult>("cleanup_path_entries", { confirmationToken: token.token });
    }, "正在清理 PATH");
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
    state.pythonRepairPlan = null;
    renderPythonAnalysis();
    renderPythonRepairPlan();
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

function errorToText(error: unknown) {
  if (error instanceof Error) return `${error.name}: ${error.message}${error.stack ? `\n${error.stack}` : ""}`;
  return String(error);
}

function safetyDisclaimerAccepted() {
  const settings = state.config?.settings;
  return Boolean(
    settings?.safetyDisclaimerAccepted &&
      (settings.safetyDisclaimerVersion || 0) >= SAFETY_DISCLAIMER_VERSION,
  );
}

function renderSafetyGate() {
  const gate = document.querySelector<HTMLElement>("#safety-gate");
  if (!gate) return;
  if (safetyDisclaimerAccepted()) {
    gate.hidden = true;
    gate.innerHTML = "";
    document.body.classList.remove("modal-locked");
    return;
  }
  document.body.classList.add("modal-locked");
  gate.hidden = false;
  gate.innerHTML = `
    <section class="safety-gate-card" role="dialog" aria-modal="true" aria-labelledby="safety-gate-title">
      <div>
        <h2 id="safety-gate-title">使用前请阅读安全声明</h2>
        <p>DevEnv Manager 会展示诊断证据，并在你确认后执行环境变量、进程、服务、文件或数据库相关操作。确认前主界面不可操作。</p>
      </div>
      <pre>${escapeHtml(state.safetyDisclaimer || "修改环境、结束进程、清理文件或修复数据库前，请确认对象、备份重要数据，并理解失败后的恢复方式。")}</pre>
      <div class="safety-gate-actions">
        <button data-action="copy-safety-disclaimer">${icon(Clipboard)}<span>复制声明</span></button>
        <button id="accept-safety-disclaimer" class="primary">${icon(Shield)}<span>我已阅读并知晓风险</span></button>
      </div>
    </section>
  `;
}

function renderFatalError() {
  const element = document.querySelector<HTMLElement>("#fatal-error");
  if (!element) return;
  if ((!state.safeMode && !state.fatalError) || state.safeModeNoticeCollapsed) {
    element.hidden = true;
    element.innerHTML = "";
    return;
  }
  element.hidden = false;
  element.innerHTML = `
    <section class="fatal-error-card">
      <div class="fatal-error-heading">
        <div>
          <h2>已进入安全模式</h2>
          <p>${escapeHtml(SAFE_MODE_DESCRIPTION)}</p>
        </div>
        <button data-action="dismiss-safe-mode-banner" aria-label="收起安全模式提示">×</button>
      </div>
      <pre>${escapeHtml(state.fatalError || "未知错误")}</pre>
      <div class="fatal-error-actions">
        <button data-action="retry-app-init">${icon(RefreshCw)}<span>重试</span></button>
        <button data-action="reset-ui-config">${icon(RotateCcw)}<span>重置 UI 配置</span></button>
        <button data-action="open-app-config-dir">${icon(FolderOpen)}<span>打开配置目录</span></button>
        <button data-action="copy-diagnostics">${icon(Clipboard)}<span>复制诊断信息</span></button>
      </div>
    </section>
  `;
}

function enterSafeMode(error: unknown, context = "运行时错误") {
  state.safeMode = true;
  state.fatalError = `${context}\n${errorToText(error)}`;
  state.safeModeNoticeCollapsed = false;
  renderFatalError();
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
      ${paginate("wsl-items", report.wslItems, (item) => `
        <article class="runtime">
          <div><strong>${escapeHtml(item.name)}</strong><span>${item.isDefault ? "默认 · " : ""}${escapeHtml(item.state)} · WSL ${escapeHtml(item.version)}</span></div>
          <div class="row-actions">
            <button data-action="system-platform" data-platform-action="wsl_start" data-platform-value="${escapeHtml(item.name)}">${icon(Play)}<span>启动</span></button>
            <button data-action="system-platform" data-platform-action="wsl_set_default" data-platform-value="${escapeHtml(item.name)}">${icon(RefreshCw)}<span>设为默认</span></button>
            <button data-action="system-platform" data-platform-action="wsl_terminate" data-platform-value="${escapeHtml(item.name)}">${icon(Trash2)}<span>终止</span></button>
          </div>
        </article>
      `) || `<div class="empty">没有发现 WSL 发行版</div>`}
    </div>
  `;
}

function renderLocalServices() {
  const element = document.querySelector<HTMLElement>("#local-service-result");
  if (!element) return;
  element.innerHTML = state.localServices.length
    ? paginate("local-services", state.localServices,
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
          `)
    : `<div class="empty">尚未检查常见开发服务</div>`;
}

function renderMySqlRepair() {
  const element = document.querySelector<HTMLElement>("#mysql-repair-result");
  const report = state.mysqlRepair;
  if (!element) return;
  if (!report) {
    element.innerHTML = `<div class="empty">检查服务丢失、1067 线索、my.ini、Data 系统库和候选业务库</div>`;
    return;
  }
  element.innerHTML = `<div class="scan-only-banner">${icon(Shield)}<span>${escapeHtml(report.privacyNotice)}</span></div>
    ${report.warnings.map((item) => `<p class="small-note">${escapeHtml(item)}</p>`).join("")}
    <div class="runtime-list">${report.candidates.length ? report.candidates.map((candidate) => `
      <article class="runtime mysql-candidate ${candidate.conclusionLevel === "Healthy" ? "ok" : "warn"}">
        <div><strong>${escapeHtml(candidate.serviceName)} · MySQL ${escapeHtml(candidate.versionHint)}</strong><span>${escapeHtml(candidate.conclusionLevel)} / ${escapeHtml(candidate.serviceState)}</span></div>
        ${candidate.conclusionLevel === "PermissionUnknown" ? `<div class="advanced-warning">${icon(Shield)}<span>${escapeHtml(MYSQL_PERMISSION_UNKNOWN_HELP)}</span></div>` : ""}
        <div class="mysql-sections">
          <section><h3>概览</h3><div class="kv-list toolchain-kv"><div><span>服务名</span><strong>${escapeHtml(candidate.serviceName)}</strong></div><div><span>服务状态</span><strong>${escapeHtml(candidate.serviceState)}</strong></div><div><span>端口</span><strong>${candidate.port} · ${candidate.portOccupied ? "已占用" : "空闲"}</strong></div><div><span>端口占用进程</span><strong>${escapeHtml(candidate.portProcess)}</strong></div><div><span>结论可信度</span><strong>${escapeHtml(candidate.confidence)}</strong></div></div></section>
          <section><h3>证据</h3><div class="kv-list toolchain-kv">${mysqlPathValue("mysqld", candidate.mysqldPath, icon(Clipboard), escapeHtml)}${mysqlPathValue("my.ini", candidate.myIniPath, icon(Clipboard), escapeHtml)}${mysqlPathValue("basedir", candidate.basedir, icon(Clipboard), escapeHtml)}${mysqlPathValue("datadir", candidate.datadir, icon(Clipboard), escapeHtml)}<div><span>静态文件检查</span><strong>${escapeHtml(candidate.staticFileCheck)}</strong></div><div><span>连接验证</span><strong>${escapeHtml(candidate.connectionCheck)}</strong></div><div><span>系统 schema</span><strong>${escapeHtml(candidate.systemSchemaCheck)}</strong></div><div><span>业务库候选</span><strong>${escapeHtml(candidate.businessDatabases.join("、") || "未发现")}</strong></div></div></section>
          <section><h3>风险</h3><ul>${candidate.reasoning.map((item) => `<li>${escapeHtml(item)}</li>`).join("")}${candidate.suggestions.map((item) => `<li>${escapeHtml(item)}</li>`).join("")}</ul></section>
          <section><h3>备份</h3>${renderMySqlBackupManifest(candidate.backupManifest)}</section>
        </div>
        ${candidate.lastError ? `<details><summary>最近错误摘要（最多 80 行）</summary><pre class="command-output compact-output">${escapeHtml(candidate.lastError)}</pre></details>` : ""}
        <div class="row-actions"><button data-mysql-action="backup" data-candidate="${candidate.id}">备份 Data</button>${candidate.status === "NotInstalled" ? `<button data-mysql-action="register_service" data-candidate="${candidate.id}">预览注册服务</button>` : ""}${candidate.serviceState.toLowerCase() !== "running" && candidate.status !== "NotInstalled" ? `<button data-mysql-action="start_service" data-candidate="${candidate.id}">预览启动</button>` : ""}${canRepairMySqlSystemSchema(candidate) ? `<button data-mysql-action="repair_system_schema" data-candidate="${candidate.id}" class="danger-button">预览补回系统库</button>` : ""}<button data-mysql-action="reset_root_guide" data-candidate="${candidate.id}">认证恢复向导</button>${candidate.businessDatabases.length ? `<button data-mysql-action="dump_guide" data-candidate="${candidate.id}">导出建议</button>` : ""}<button data-action="copy-text" data-copy="${escapeHtml(candidate.consoleCommand)}">复制控制台诊断命令</button></div>
        ${candidate.systemSchemaMissing && !canRepairMySqlSystemSchema(candidate) ? `<p class="small-note warning-text">系统库修复暂不可用：${escapeHtml(mysqlRepairBlockReason(candidate))}</p>` : ""}
      </article>`).join("") : `<div class="empty">常见安装位置没有发现 mysqld.exe；不会自动深扫整个磁盘</div>`}</div>`;
}

function renderMySqlBackupManifest(manifest?: MySqlBackupManifestStatus | null) {
  if (!manifest) return `<div class="empty">还没有本程序登记的 backup manifest；高危修复前必须先备份 Data。</div>`;
  return `<div class="kv-list toolchain-kv">
    <div><span>状态</span><strong>${manifest.valid ? "有效" : "无效"} · ${escapeHtml(manifest.reason)}</strong></div>
    <div><span>最近备份时间</span><strong>${new Date(manifest.createdAt * 1000).toLocaleString("zh-CN")}</strong></div>
    <div><span>有效期至</span><strong>${new Date(manifest.expiresAt * 1000).toLocaleString("zh-CN")}</strong></div>
    <div><span>备份目录</span><strong>${escapeHtml(manifest.destination)}</strong></div>
    <div><span>文件/大小</span><strong>${manifest.files} 个 · ${formatBytes(manifest.bytes)}</strong></div>
    <div><span>关键文件</span><strong>ibdata1 ${manifest.ibdata ? "存在" : "未见"} · 系统库 ${manifest.systemSchema ? "存在" : "未见"} · 业务库 ${manifest.businessSchema ? "存在" : "未见"}</strong></div>
    <div><span>manifest</span><strong>${escapeHtml(manifest.manifestPath)}</strong></div>
  </div>`;
}

function canRepairMySqlSystemSchema(candidate: MySqlCandidate) {
  return candidate.systemSchemaMissing && Boolean(candidate.backupManifest?.valid) && ["LikelyBroken", "PotentialRisk"].includes(candidate.conclusionLevel);
}

function mysqlRepairBlockReason(candidate: MySqlCandidate) {
  if (!["LikelyBroken", "PotentialRisk"].includes(candidate.conclusionLevel)) return `当前结论为 ${candidate.conclusionLevel}`;
  if (!candidate.backupManifest) return "没有 backup manifest";
  if (!candidate.backupManifest.valid) return candidate.backupManifest.reason;
  return "请重新诊断后再试";
}

function renderMySqlPlan() {
  const element = document.querySelector<HTMLElement>("#mysql-plan-preview");
  const plan = state.mysqlPlan;
  if (!element) return;
  if (!plan) {
    element.innerHTML = `<div class="empty">选择候选与动作后显示一次性修复计划</div>`;
    return;
  }
  element.innerHTML = `<article class="repair-plan-card"><div><strong>${escapeHtml(plan.title)}</strong><span>${plan.requiresAdmin ? "需要管理员权限" : "当前用户权限"}${plan.requiresBackup ? " · 强制先备份" : ""}</span></div><ol>${plan.steps.map((item) => `<li>${escapeHtml(item)}</li>`).join("")}</ol><pre class="command-output compact-output">${escapeHtml(plan.commands.join("\n"))}</pre><ul>${plan.warnings.map((item) => `<li>${escapeHtml(item)}</li>`).join("")}</ul>${plan.action === "backup" ? `<input id="mysql-backup-destination" placeholder="备份目标，例如 D:\\DevEnvManagerBackups\\mysql-data-20260624" />` : ""}<button id="execute-mysql-plan" class="${plan.action === "reset_root_guide" || plan.action === "dump_guide" ? "" : "danger-button"}">${plan.action === "reset_root_guide" || plan.action === "dump_guide" ? "生成只读向导" : "二次确认并执行计划"}</button></article>`;
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
    ? paginate("download-cache", state.cache,
          (item) => `
            <article class="runtime">
              <div><strong>${escapeHtml(item.name)}</strong><span>${formatBytes(item.size)}</span></div>
              <small>${escapeHtml(item.sha256 || item.path)}</small>
            </article>
          `)
    : `<div class="empty">下载缓存为空</div>`;
}

function renderUpdate() {
  const elements = Array.from(document.querySelectorAll<HTMLElement>("[data-update-result]"));
  const update = state.update;
  if (!elements.length) return;
  const html = !update
    ? updateEmptyState(state.updateError, escapeHtml)
    : `
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
  elements.forEach((element) => {
    element.innerHTML = html;
  });
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
      ${paginate("agent-traces", report.items, (item) => `
        <article class="runtime">
          <div><strong>${escapeHtml(item.source)}</strong><span>置信度：${escapeHtml(item.confidence)}</span></div>
          <small>${escapeHtml(item.path)}</small>
          <small>${escapeHtml(item.evidence)}</small>
          <small>${escapeHtml(item.recommendation)}</small>
        </article>
      `) || `<div class="empty">没有发现可验证的 Agent / CLI 安装痕迹</div>`}
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
      <article class="maintenance-metric"><span>可清理空间估算</span><strong>${formatBytes(overview.safeCleanEstimate)}</strong><small>需扫描、选择并确认计划</small></article>
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

function renderScanCategories(target: string, categoryIds: string[], selectable = false) {
  const element = document.querySelector<HTMLElement>(target);
  if (!element) return;
  const report = state.cleanupReport;
  if (!report) return;
  const categories = report.categories.filter((category) => categoryIds.includes(category.id));
  element.innerHTML = `
    <div class="scan-summary"><strong>${formatBytes(categories.reduce((sum, item) => sum + item.totalBytes, 0))}</strong><span>${categories.reduce((sum, item) => sum + item.itemCount, 0)} 个扫描项</span></div>
    <div class="maintenance-category-list">
      ${categories.map((category) => `
        <details class="maintenance-category" ${category.totalBytes ? "open" : ""}>
          <summary>
            <span><strong>${escapeHtml(category.name)}</strong><small>${escapeHtml(category.description)}</small></span>
            <span><b>${formatBytes(category.totalBytes)}</b><i class="risk-chip risk-${escapeHtml(category.risk)}">${riskText(category.risk)}风险</i></span>
          </summary>
          <div class="maintenance-items">
            ${paginate(`cleanup-${target}-${category.id}`, category.items, (item) => `
              <article class="maintenance-item">
                <div>${selectable && item.cleanable ? `<label class="cleanup-check"><input type="checkbox" data-cleanup-item="${escapeHtml(item.id)}" ${state.cleanupSelection.has(item.id) ? "checked" : ""} /><strong>${escapeHtml(item.source)}</strong></label>` : `<strong>${escapeHtml(item.source)}</strong>`}<span>${formatBytes(item.size)}</span></div>
                <small>${escapeHtml(item.path)}</small>
                <small>${escapeHtml(item.skippedReason || item.reason)} · ${item.cleanable ? "可加入清理计划" : "只读扫描，不建议自动清理"}</small>
              </article>
            `) || `<div class="empty">目录不存在或占用为 0</div>`}
          </div>
        </details>
      `).join("")}
    </div>
    <ul class="scan-warnings">${report.warnings.map((warning) => `<li>${escapeHtml(warning)}</li>`).join("")}</ul>
  `;
}

function renderMaintenanceScan() {
  renderScanCategories("#maintenance-cleanup-categories", ["windows-temp", "devenv-manager"], true);
  renderScanCategories("#maintenance-expert-categories", ["system-caches", "recycle-bin", "wps-cache"]);
  renderScanCategories("#maintenance-dev-categories", ["developer-caches"]);
  const preview = document.querySelector<HTMLButtonElement>("#preview-cleanup-plan");
  if (preview) preview.disabled = state.cleanupSelection.size === 0;
}

function renderEnvironmentPreview() {
  const element = document.querySelector<HTMLElement>("#env-config-preview");
  const preview = state.environmentPreview;
  if (!element) return;
  element.innerHTML = preview
    ? `<section class="environment-preview"><div class="panel-head"><div class="panel-title">${icon(Shield)}<h3>环境配置差异</h3></div><span>备份：${escapeHtml(preview.backupName)}</span></div>
       <div class="runtime-list">${preview.changes.map((change) => `<article class="runtime"><div><strong>${escapeHtml(change.name)}</strong><span>${escapeHtml(change.current || "未设置")} → ${escapeHtml(change.proposed || "不设置")}</span></div><small>${escapeHtml(change.impact)}</small></article>`).join("")}</div>
       <div class="grid two"><div><h4>PATH 新增</h4>${preview.pathAdded.length ? `<ul>${preview.pathAdded.map((item) => `<li><code>${escapeHtml(item)}</code></li>`).join("")}</ul>` : `<div class="empty">没有新增条目</div>`}</div><div><h4>PATH 移除</h4>${preview.pathRemoved.length ? `<ul>${preview.pathRemoved.map((item) => `<li><code>${escapeHtml(item)}</code></li>`).join("")}</ul>` : `<div class="empty">不会移除外部路径</div>`}</div></div>
       <ul>${preview.warnings.map((warning) => `<li>${escapeHtml(warning)}</li>`).join("")}</ul>
       <button id="apply-environment-preview" class="primary">二次确认并写入</button></section>`
    : `<div class="empty">点击“预览配置”查看 DEVENV_HOME、JAVA_HOME 和 PATH 的实际差异</div>`;
}

function renderEnvironmentBackups() {
  const element = document.querySelector<HTMLElement>("#env-backup-list");
  if (!element) return;
  element.innerHTML = state.environmentBackups.length
    ? paginate("environment-backups", state.environmentBackups, (backup) => `<article class="runtime"><div><strong>${escapeHtml(backup.fileName)}</strong><span>${backup.pathEntries} 个 PATH 条目</span></div><small>DEVENV_HOME：${escapeHtml(backup.devenvHome || "未设置")} · JAVA_HOME：${escapeHtml(backup.javaHome || "未设置")}</small><div class="row-actions"><button data-restore-env-backup="${escapeHtml(backup.fileName)}">恢复此备份</button></div></article>`)
    : `<div class="empty">还没有环境备份；首次应用配置时会自动创建</div>`;
}

async function sha256Hex(text: string) {
  const bytes = new TextEncoder().encode(text);
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  return Array.from(new Uint8Array(digest)).map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

async function createBackendConfirmation(
  actionId: string,
  planId: string,
  riskLevel: string,
  planFingerprint: string,
  tripleConfirmed: boolean,
  backupReceipt?: string | null,
  command?: string,
) {
  return invoke<ConfirmationTokenView>("create_confirmation_token", {
    command: command || actionId,
    actionId,
    planId,
    riskLevel,
    planFingerprint,
    tripleConfirmed,
    backupReceipt: backupReceipt || null,
  });
}

async function processActionFingerprint(actionId: string, planId: string, riskLevel: string) {
  return sha256Hex(`${actionId}\0${planId}\0${riskLevel}`);
}

async function riskOperationToken(
  command: string,
  planId: string,
  riskLevel: "medium" | "high" | "critical",
  tripleConfirmed = false,
  backupReceipt: string | null = null,
  actionId = command,
) {
  const fingerprint = await sha256Hex(`${command}\0${planId}\0${riskLevel}`);
  return createBackendConfirmation(actionId, planId, riskLevel, fingerprint, tripleConfirmed, backupReceipt, command);
}

function renderSafetyDisclaimer() {
  const slot = document.querySelector<HTMLElement>("#safety-disclaimer-slot");
  if (!slot) return;
  const accepted = safetyDisclaimerAccepted();
  slot.innerHTML = accepted ? "" : disclaimerPanel(escapeHtml(state.safetyDisclaimer || "执行修改类操作前请先阅读风险说明，并备份重要数据。"));
}

function renderEnvReliability() {
  const element = document.querySelector<HTMLElement>("#env-reliability-result");
  const snapshot = state.envReliability;
  if (!element || !snapshot) return;
  const pathEntries = snapshot.userEnv.pathEntries.filter((entry) => entry.containsJava || entry.containsJavac || entry.containsPython || entry.containsPip || entry.isDuplicate || entry.isStaleDevenvEntry).slice(0, 12);
  element.innerHTML = `
    <div class="grid two env-reliability-grid">
      <article class="runtime"><div><strong>JAVA_HOME</strong>${riskBadge(snapshot.java.consistency === "ok" ? "info" : snapshot.java.consistency)}</div><small>raw：${escapeHtml(snapshot.java.javaHomeRaw || "未设置")}</small><small>expanded：${escapeHtml(snapshot.java.javaHomeExpanded || "未设置")}</small></article>
      <article class="runtime"><div><strong>PATH 首个 Java</strong><span>${snapshot.java.javaHomeValid ? "JDK 根目录有效" : "需要修复"}</span></div><small>java：${escapeHtml(snapshot.java.pathJava || "未发现")}</small><small>javac：${escapeHtml(snapshot.java.pathJavac || "未发现")}</small></article>
      <article class="runtime"><div><strong>Java / javac 版本</strong><span>只读验证</span></div><small>${escapeHtml(snapshot.java.commandJavaVersion || "java 无输出")}</small><small>${escapeHtml(snapshot.java.commandJavacVersion || "javac 无输出")}</small></article>
      <article class="runtime"><div><strong>Python / pip</strong>${riskBadge(snapshot.python.pipMatchesPython ? "info" : "medium")}</div><small>Python：${escapeHtml(snapshot.python.currentPython?.path || "未发现")}</small><small>pip：${escapeHtml(snapshot.python.currentPip?.path || "未发现")} · ${snapshot.python.discoveredPythons.length} 个 Python / ${snapshot.python.discoveredPips.length} 个 pip</small></article>
      <article class="runtime"><div><strong>Maven / Gradle 使用 Java</strong><span>可选工具</span></div><small>Maven：${escapeHtml(snapshot.mavenGradle.mavenJava || snapshot.mavenGradle.mavenVersion || "未安装/未读取")}</small><small>Gradle：${escapeHtml(snapshot.mavenGradle.gradleJava || snapshot.mavenGradle.gradleVersion || "未安装/未读取")}</small></article>
      <article class="runtime"><div><strong>PATH 质量</strong>${riskBadge(snapshot.pathAnalysis.staleDevenvCount || snapshot.pathAnalysis.duplicateCount ? "medium" : "info")}</div><small>${snapshot.pathAnalysis.totalEntries} 项；重复 ${snapshot.pathAnalysis.duplicateCount}；失效 ${snapshot.pathAnalysis.missingCount}；旧 DevEnv ${snapshot.pathAnalysis.staleDevenvCount}</small><small>Java 入口 ${snapshot.pathAnalysis.javaEntryCount}；Python 入口 ${snapshot.pathAnalysis.pythonEntryCount}</small></article>
    </div>
    <section class="runtime-list">${pathEntries.map((entry) => `<article class="runtime"><div><strong>${escapeHtml(entry.raw)}</strong>${riskBadge(entry.risk)}</div><small>${escapeHtml(entry.expanded)}</small><small>${entry.exists ? "存在" : "不存在"}${entry.isDuplicate ? " · 重复" : ""}${entry.isStaleDevenvEntry ? " · 旧 DevEnv 受管残留" : ""}</small></article>`).join("") || `<div class="empty">没有需要特别关注的 PATH 项</div>`}</section>
    <ul class="scan-warnings">${[...envReliabilityIntro(), ...snapshot.java.conflicts, ...snapshot.python.conflicts, ...snapshot.mavenGradle.conflicts].map((item) => `<li>${escapeHtml(item)}</li>`).join("")}</ul>
  `;
}

function renderEnvRepairPlan() {
  const element = document.querySelector<HTMLElement>("#env-repair-plan-result");
  const button = document.querySelector<HTMLButtonElement>("#apply-env-repair-plan");
  if (!element || !button) return;
  const plan = state.envRepairPlan;
  button.disabled = !plan;
  element.innerHTML = plan
    ? `<article class="runtime"><div><strong>${escapeHtml(plan.target)} · ${plan.actions.length} 个动作</strong>${riskBadge(plan.riskLevel)}</div><small>备份：${escapeHtml(plan.backupName)} · 需要重启终端：${plan.requiresTerminalRestart ? "是" : "否"}</small><small>${escapeHtml(plan.disclaimer)}</small>${plan.diff.length ? `<pre class="command-output compact-output">${escapeHtml(plan.diff.join("\n"))}</pre>` : ""}</article>
       <div class="runtime-list">${plan.actions.map((action) => `<article class="runtime"><div><strong>${escapeHtml(action.title)}</strong>${riskBadge(action.risk)}</div><small>${escapeHtml(action.description)}</small><small>${escapeHtml(action.oldValue || "未设置")} → ${escapeHtml(action.newValue || "不设置")}</small></article>`).join("")}</div>
       ${state.envRepairResult ? `<article class="runtime"><div><strong>${state.envRepairResult.success ? "验证通过" : "验证有警告"}</strong><span>备份 ${escapeHtml(state.envRepairResult.backupName)}</span></div><small>${escapeHtml(state.envRepairResult.message)}</small></article>` : ""}`
    : `<div class="empty">修复计划会展示 diff、备份名、风险说明和验证结果。</div>`;
}

function renderEnvBackupRecords() {
  const element = document.querySelector<HTMLElement>("#env-backup-records");
  if (!element) return;
  element.innerHTML = state.envBackupRecords.length
    ? paginate("env-backup-records", state.envBackupRecords, (record) => `<article class="runtime"><div><strong>${escapeHtml(record.backupName)}</strong><span>${record.pathEntryCount} 个 PATH 条目</span></div><small>${escapeHtml(record.reason)} · JAVA_HOME：${escapeHtml(record.javaHomePreview || "未设置")}</small><div class="row-actions"><button data-action="restore-env-record" data-backup-name="${escapeHtml(record.backupName)}">二次确认恢复</button></div></article>`, 10)
    : `<div class="empty">暂无 Phase 5 环境备份</div>`;
}

function runtimeSwitchOptions(items: ManagedRuntime[], current?: string | null) {
  return `<option value="">不切换${current ? `（当前 ${escapeHtml(current)}）` : ""}</option>${items.map((item) => `<option value="${escapeHtml(item.version)}">${escapeHtml(item.version)}</option>`).join("")}`;
}

function renderProjectConfigPreview() {
  const element = document.querySelector<HTMLElement>("#project-config-preview");
  const preview = state.projectConfigPreview;
  const installed = state.config?.installed;
  if (!element || !preview || !installed) return;
  element.innerHTML = `
    <div class="panel-head"><div class="panel-title">${icon(Hammer)}<h2>配置与环境切换预览</h2></div><span>${escapeHtml(preview.detectedTypes.join(" / ") || "未识别类型")}</span></div>
    <div class="project-config-files">${preview.files.map((file, index) => `<article class="project-config-file"><label><input type="checkbox" data-project-file-enabled="${index}" ${file.enabled ? "checked" : ""} /><strong>${escapeHtml(file.relativePath)}</strong><span>${file.existed ? "将备份后更新" : "将新建"}</span></label><textarea data-project-file-content="${index}" spellcheck="false">${escapeHtml(file.content)}</textarea></article>`).join("")}</div>
    <h3>可选运行时切换</h3>
    <div class="runtime-switch-grid">
      <label>JDK<select data-project-switch="jdk">${runtimeSwitchOptions(installed.jdks, installed.current.jdk)}</select></label>
      <label>Python<select data-project-switch="python">${runtimeSwitchOptions(installed.pythons, installed.current.python)}</select></label>
      <label>Node.js<select data-project-switch="node">${runtimeSwitchOptions(installed.nodes, installed.current.node)}</select></label>
      <label>Maven<select data-project-switch="maven">${runtimeSwitchOptions(installed.mavens, installed.current.maven)}</select></label>
      <label>Gradle<select data-project-switch="gradle">${runtimeSwitchOptions(installed.gradles, installed.current.gradle)}</select></label>
      <label>Go<select data-project-switch="go">${runtimeSwitchOptions(installed.gos, installed.current.go)}</select></label>
    </div>
    <ul>${preview.warnings.map((warning) => `<li>${escapeHtml(warning)}</li>`).join("")}</ul>
    <button id="apply-project-config" class="primary">二次确认并应用</button>
  `;
}

function renderCleanupPlan() {
  const element = document.querySelector<HTMLElement>("#cleanup-plan-preview");
  const execute = document.querySelector<HTMLButtonElement>("#execute-cleanup-plan");
  if (!element || !execute) return;
  const plan = state.cleanupPlan;
  execute.disabled = !plan;
  element.innerHTML = plan
    ? `<div class="panel-head"><div class="panel-title">${icon(Shield)}<h2>清理计划预览</h2></div><span>${plan.selectedItems.length} 项 · ${formatBytes(plan.estimatedBytes)}</span></div>
       <div class="runtime-list">${paginate("cleanup-plan", plan.selectedItems, (item) => `<article class="runtime"><div><strong>${escapeHtml(item.categoryId)}</strong><span>${formatBytes(item.size)} · ${riskText(item.risk)}风险</span></div><small>${escapeHtml(item.path)}</small><small>${item.reversible ? "移入回收站，可恢复" : "官方命令，缓存将重新生成"}</small></article>`)}</div>
       ${plan.warnings.length ? `<ul>${plan.warnings.map((warning) => `<li>${escapeHtml(warning)}</li>`).join("")}</ul>` : ""}`
    : `<div class="empty">选择项目后点击“预览清理计划”</div>`;
}

function renderCleanupResult() {
  const targets = ["#cleanup-report", "#cleanup-plan-preview"];
  const result = state.cleanupResult;
  if (!result) return;
  const html = `<div class="cleanup-result ${result.success ? "ok" : "warn"}">
    <h2>${result.success ? "清理完成" : "清理部分完成"}</h2>
    <div class="maintenance-metrics"><article class="maintenance-metric"><span>释放空间</span><strong>${formatBytes(result.cleanedBytes)}</strong></article><article class="maintenance-metric"><span>完成</span><strong>${result.cleanedItems}</strong></article><article class="maintenance-metric"><span>跳过</span><strong>${result.skippedItems}</strong></article><article class="maintenance-metric"><span>失败</span><strong>${result.failedItems}</strong></article></div>
    ${result.failures.length ? `<ul>${result.failures.map((failure) => `<li><code>${escapeHtml(failure.path)}</code>：${escapeHtml(failure.reason)}</li>`).join("")}</ul>` : ""}
    <div class="toolbar"><button data-cleanup-report-action="copy">复制 Markdown 报告</button><button data-cleanup-report-action="json">导出 JSON 报告</button></div>
  </div>`;
  targets.forEach((target) => {
    const element = document.querySelector<HTMLElement>(target);
    if (element) element.innerHTML = html;
  });
}

function renderFolderUsage(target: string, report: FolderUsageReport | null, key: string) {
  const element = document.querySelector<HTMLElement>(target);
  if (!element || !report) return;
  const rescanTarget = key.includes("desktop") ? "desktop" : "downloads";
  const categoryKey = `${key}-categories`;
  const topFileKey = `${key}-top-files`;
  element.innerHTML = `
    <section class="folder-usage-summary">
      <div>
        <strong>${escapeHtml(report.name)}</strong>
        <span>${formatBytes(report.totalBytes)}</span>
      </div>
      <small title="${escapeHtml(report.path)}">${escapeHtml(report.path)}</small>
      <div class="folder-usage-notes">${[...report.suggestions, ...report.warnings].map((item) => `<span>${escapeHtml(item)}</span>`).join("")}</div>
    </section>
    <section class="folder-usage-section">
      <div class="section-heading">
        <div><h3>分类占用</h3><small>按文件类型、时间和用途分组；展开分类可查看该类 Top 文件。</small></div>
        <span>${report.categories.length} 类</span>
      </div>
      <div class="folder-usage-grid">${paginate(categoryKey, report.categories, (category) => `<details class="folder-usage-card"><summary><div><strong>${escapeHtml(category.name)}</strong><span>${formatBytes(category.size)}</span></div><small>${escapeHtml(category.suggestion)}</small></summary><div class="runtime-list compact-file-list">${category.details.length ? paginate(`${categoryKey}-${category.name}`, category.details, (item) => renderFileDetail(item, rescanTarget), 4) : `<div class="empty">这个分类下没有可展示的 Top 文件明细</div>`}</div></details>`, 4)}</div>
    </section>
    <section class="folder-usage-section top-files-section">
      <div class="section-heading">
        <div><h3>Top 文件明细</h3><small>优先展示最占空间的具体文件；每页最多 6 项，便于逐个定位和复制路径。</small></div>
        <button data-action="rescan-folder" data-target="${rescanTarget}">${icon(RefreshCw)}<span>重新扫描</span></button>
      </div>
      <div class="file-detail-grid">${report.topFiles.length ? paginate(topFileKey, report.topFiles, (item) => renderFileDetail(item, rescanTarget), 6) : `<div class="empty">没有可展示的文件明细</div>`}</div>
    </section>
  `;
}

function renderFileDetail(item: LargeFileItem, rescanTarget: string) {
  const directory = fileDirectory(item.path, item.directory);
  const modified = item.modifiedAt || "未知修改时间";
  const extension = item.extension || "无扩展名";
  const locateLabel = item.exists ? "选中文件" : "尝试定位";
  return `<article class="runtime file-detail-card ${item.exists ? "" : "missing-file"}">
    <div class="file-detail-head">
      <strong title="${escapeHtml(item.fileName || item.path)}">${escapeHtml(item.fileName || item.path)}</strong>
      <span>${formatBytes(item.size)}</span>
    </div>
    <div class="file-detail-meta">
      <span>${escapeHtml(extension)}</span>
      <span>${escapeHtml(item.fileType)}</span>
      <span>${escapeHtml(item.sourceCategory)}</span>
      <span>${escapeHtml(modified)}</span>
      <span>${item.exists ? "仍存在" : "已移动或删除"}</span>
      <span>${item.canLocate ? "可定位" : "不可定位"}</span>
    </div>
    <small title="${escapeHtml(item.path)}">完整路径：${escapeHtml(item.path)}</small>
    <small title="${escapeHtml(directory)}">所在目录：${escapeHtml(directory)}</small>
    <small>${escapeHtml(item.openStatus || item.suggestion)}</small>
    <div class="row-actions file-actions">
      <button data-action="open-analysis-path" data-path="${escapeHtml(directory)}" ${item.canLocate ? "" : "disabled"}>打开所在目录</button>
      <button data-action="open-analysis-path" data-path="${escapeHtml(item.path)}" ${item.canOpen || item.canLocate ? "" : "disabled"}>${locateLabel}</button>
      <button data-action="copy-text" data-copy="${escapeHtml(item.path)}">复制完整路径</button>
      <button data-action="copy-text" data-copy="${escapeHtml(directory)}">复制所在目录</button>
      <button data-action="rescan-folder" data-target="${escapeHtml(rescanTarget)}">重新扫描</button>
    </div>
  </article>`;
}

function renderLargeFiles() {
  const element = document.querySelector<HTMLElement>("#large-file-result");
  if (!element) return;
  element.innerHTML = state.largeFiles.length
    ? paginate("large-files", state.largeFiles, (item) => `${renderFileDetail(item, "large-files")}<div class="row-actions inline-archive-action"><button data-action="archive-add" data-path="${escapeHtml(item.path)}" data-source="大文件">加入归档计划</button></div>`, 10)
    : `<div class="empty">扫描范围内没有达到阈值的大文件</div>`;
}

function renderArchivePlan() {
  const element = document.querySelector<HTMLElement>("#archive-plan-list");
  if (!element) return;
  element.innerHTML = state.archivePlan.length
    ? paginate("archive-plan", state.archivePlan, (item) => `<article class="runtime"><div><strong>${escapeHtml(item.source)} · ${formatBytes(item.size)}</strong><span>仅计划</span></div><small>${escapeHtml(item.path)}</small><small>${escapeHtml(item.suggestion)}</small><div class="row-actions"><button data-action="open-analysis-path" data-path="${escapeHtml(item.path)}">打开位置</button><button data-action="archive-remove" data-archive-id="${escapeHtml(item.id)}">移出计划</button></div></article>`, 10)
    : `<div class="empty">归档计划为空；先从大文件/重复文件结果加入候选，或生成搬家/归档计划</div>`;
}

async function loadArchivePlan() {
  state.archivePlan = await invoke<ArchivePlanItem[]>("list_archive_plan_items");
  renderArchivePlan();
}

function renderMovePlan() {
  const element = document.querySelector<HTMLElement>("#move-plan-result");
  const execute = document.querySelector<HTMLButtonElement>("#execute-move-plan");
  if (!element || !execute) return;
  const plan = state.movePlan;
  execute.disabled = !plan;
  const result = state.moveResult;
  element.innerHTML = plan
    ? `<article class="runtime move-plan-card">
        <div><strong>${escapeHtml(plan.mode)} · ${formatBytes(plan.estimatedBytes)}</strong><span class="risk-chip risk-${escapeHtml(plan.risk)}">${riskText(plan.risk)}风险</span></div>
        <small>源：${escapeHtml(plan.source)}</small>
        <small>目标：${escapeHtml(plan.target)}</small>
        <small>${plan.itemCount} 个文件 · ${plan.reversible ? "可自动回滚" : "归档需按报告手动恢复"}</small>
        ${plan.warnings.length ? `<ul>${plan.warnings.map((warning) => `<li>${escapeHtml(warning)}</li>`).join("")}</ul>` : ""}
      </article>
      ${result ? `<article class="runtime">
        <div><strong>${result.success ? "执行成功" : "执行未完全成功"} · ${formatBytes(result.movedBytes)}</strong><span>${result.movedItems} 项</span></div>
        <small>目标：${escapeHtml(result.targetPath)}</small>
        <small>备份：${escapeHtml(result.sourceBackup || "无")} · 回滚 ID：${escapeHtml(result.rollbackId || "无")}</small>
        ${result.failures.length ? `<ul>${result.failures.map((failure) => `<li>${escapeHtml(failure)}</li>`).join("")}</ul>` : "<small>无失败项</small>"}
        <pre class="command-output compact-output">${escapeHtml(result.reportMarkdown)}</pre>
      </article>` : ""}`
    : `<div class="empty">还没有空间搬家计划</div>`;
}

function renderRollbackRecords() {
  const element = document.querySelector<HTMLElement>("#rollback-records");
  if (!element) return;
  element.innerHTML = state.rollbackRecords.length
    ? paginate("rollback-records", state.rollbackRecords, (record) => `<article class="runtime">
        <div><strong>${escapeHtml(record.operationType)}</strong><span>${record.reversible ? "可回滚" : "仅报告"}</span></div>
        <small>${escapeHtml(record.source)} → ${escapeHtml(record.target)}</small>
        <small>备份：${escapeHtml(record.backupPath || "无")} · Junction：${escapeHtml(record.junctionPath || "无")}</small>
        <div class="row-actions">${record.reversible ? `<button data-action="rollback-move" data-rollback-id="${escapeHtml(record.rollbackId)}">执行回滚</button>` : ""}<button data-action="copy-text" data-copy="${escapeHtml(record.rollbackId)}">复制 ID</button></div>
      </article>`, 10)
    : `<div class="empty">暂无可自动回滚记录</div>`;
}

async function loadRollbackRecords() {
  state.rollbackRecords = await invoke<RollbackRecord[]>("list_rollback_records");
  renderRollbackRecords();
}

function renderPartitionLayout() {
  const element = document.querySelector<HTMLElement>("#partition-layout-result");
  if (!element) return;
  const report = state.partitionLayout;
  element.innerHTML = report
    ? `<section class="panel maintenance-advice">
        <div class="panel-title">${icon(Activity)}<h3>分区布局结论</h3></div>
        <p>${escapeHtml(report.explanation)}</p>
        <div class="maintenance-facts">
          <span>系统磁盘 ${escapeHtml(report.systemDisk)}</span>
          <span>C 盘 ${formatBytes(report.cPartition.size)} · ${escapeHtml(report.cPartition.fileSystem || "未知 FS")}</span>
          <span>右侧未分配 ${report.unallocatedAfterC ? formatBytes(report.unallocatedAfterC) : "无"}</span>
          <span>${report.recoveryPartitionBlocks ? "恢复分区阻挡" : "未见恢复分区阻挡"}</span>
          <span>${report.dPartitionSameDisk ? "D 盘同盘" : "D 盘不同盘或不存在"}</span>
        </div>
        ${report.adjacentRight ? `<article class="runtime"><div><strong>右侧相邻分区 ${escapeHtml(report.adjacentRight.driveLetter || `#${report.adjacentRight.partitionIndex}`)}</strong><span>${formatBytes(report.adjacentRight.size)}</span></div><small>${escapeHtml(report.adjacentRight.partitionType)} · ${report.adjacentRight.isEmpty ? "空分区候选" : "非空或未知"}</small></article>` : ""}
        <ul>${report.suggestedActions.map((item) => `<li>${escapeHtml(item)}</li>`).join("")}</ul>
      </section>`
    : `<div class="empty">尚未检测分区布局</div>`;
}

function renderExpansionPlan() {
  const element = document.querySelector<HTMLElement>("#expansion-plan-result");
  const execute = document.querySelector<HTMLButtonElement>("#execute-expansion-plan");
  if (!element || !execute) return;
  const plan = state.expansionPlan;
  execute.disabled = !plan?.canExecute;
  const result = state.expansionResult;
  element.innerHTML = plan
    ? `<article class="runtime">
        <div><strong>${escapeHtml(plan.mode)}</strong><span>${plan.canExecute ? "允许执行" : "仅说明"}</span></div>
        <small>${escapeHtml(plan.explanation)}</small>
        <small>预计新增：${formatBytes(plan.estimatedAddedBytes)} · ${plan.requiresAdmin ? "需要管理员权限" : "无需管理员权限"}</small>
        ${plan.commandsPreview.length ? `<pre class="command-output compact-output">${escapeHtml(plan.commandsPreview.join("\n"))}</pre>` : ""}
        <ul>${plan.risks.map((risk) => `<li>${escapeHtml(risk)}</li>`).join("")}</ul>
      </article>
      ${result ? `<article class="runtime"><div><strong>${result.success ? "扩容成功" : "扩容未成功"}</strong><span>${formatBytes(result.beforeTotal)} → ${formatBytes(result.afterTotal)}</span></div><pre class="command-output compact-output">${escapeHtml(result.reportMarkdown)}</pre></article>` : ""}`
    : `<div class="empty">还没有扩容计划</div>`;
}

function renderDuplicates() {
  const element = document.querySelector<HTMLElement>("#duplicate-result");
  if (!element) return;
  element.innerHTML = state.duplicateGroups.length
    ? `<div class="scan-summary"><strong>${state.duplicateGroups.length} 组</strong><span>预计可归档 ${formatBytes(state.duplicateGroups.reduce((sum, group) => sum + group.reclaimableEstimate, 0))}</span></div>${paginate("duplicate-groups", state.duplicateGroups, (group) => `<details class="maintenance-category"><summary><span><strong>${group.files.length} 个完全相同文件</strong><small>SHA256 ${escapeHtml(group.hash.slice(0, 16))}…</small></span><span><b>${formatBytes(group.size)}</b><i>可归档 ${formatBytes(group.reclaimableEstimate)}</i></span></summary><div class="runtime-list">${paginate(`duplicate-${group.hash}`, group.files, (file) => `<article class="runtime"><small>${escapeHtml(file.path)}</small><small>${escapeHtml(file.keepSuggestion)}</small><div class="row-actions"><button data-action="open-analysis-path" data-path="${escapeHtml(file.path)}">打开所在目录</button><button data-action="copy-text" data-copy="${escapeHtml(file.path)}">复制路径</button><button data-action="archive-add" data-path="${escapeHtml(file.path)}" data-source="重复文件候选">加入归档计划</button></div></article>`)}</div></details>`)} `
    : `<div class="empty">没有发现达到阈值且 SHA256 完全相同的文件</div>`;
}

function renderAppUsageItem(item: AppUsageItem) {
  return `<details class="maintenance-category"><summary><span><strong>${escapeHtml(item.name)}</strong><small>${escapeHtml(item.safeActions.join(" · "))}</small></span><span><b>${formatBytes(item.size)}</b><i class="risk-chip risk-medium">只读</i></span></summary><div class="folder-usage-grid">${item.categories.map((category) => `<article class="folder-usage-card"><div><strong>${escapeHtml(category.name)}</strong><span>${formatBytes(category.size)}</span></div><small>${escapeHtml(category.path)}</small><div class="row-actions"><button data-action="open-analysis-path" data-path="${escapeHtml(category.path)}">打开目录</button><button data-action="copy-text" data-copy="${escapeHtml(category.path)}">复制路径</button></div></article>`).join("")}</div><ul>${item.warnings.map((warning) => `<li>${escapeHtml(warning)}</li>`).join("")}</ul></details>`;
}

function renderAppUsage() {
  const element = document.querySelector<HTMLElement>("#app-usage-result");
  const report = state.appUsage;
  if (!element || !report) return;
  const applications = [
    ...(report.wechat ? [report.wechat] : []),
    ...(report.qq ? [report.qq] : []),
    ...report.browsers,
    ...report.netDisks,
    ...report.videoEditors,
    ...report.gamePlatforms,
  ].sort((a, b) => b.size - a.size);
  element.innerHTML = `<section><h3>常见应用、网盘与游戏库</h3>${applications.length ? paginate("app-usage", applications, renderAppUsageItem) : `<div class="empty">没有发现常见应用占用路径</div>`}</section>
    <section><div class="panel-head"><h3>Windows 已安装软件</h3><button data-action="open-apps-features">打开系统卸载入口</button></div><div class="runtime-list">${paginate("installed-software", report.installedSoftware, (software) => `<article class="runtime"><div><strong>${escapeHtml(software.name)}</strong><span>${software.estimatedSize ? formatBytes(software.estimatedSize) : "未登记大小"}</span></div><small>${escapeHtml(software.publisher || "未知发布者")} · ${escapeHtml(software.installLocation || "未登记安装位置")}</small><small>${escapeHtml(software.suggestion)}</small><div class="row-actions">${software.installLocation ? `<button data-action="open-analysis-path" data-path="${escapeHtml(software.installLocation)}">打开位置</button>` : ""}${software.uninstallCommandExists ? `<button data-action="open-apps-features">系统卸载</button>` : ""}</div></article>`, 10)}</div></section>`;
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
  showToast("正在执行安全扫描；此步骤不会删除任何文件");
  try {
    state.cleanupReport = await invoke<CleanupScanReport>("scan_cleanup_targets");
    state.cleanupSelection.clear();
    state.cleanupPlan = null;
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
  renderViewGuide(view, state.featureRisks, escapeHtml);
  clearFeatureHelp();
}

async function pickDirectoryInto(inputId: string, autoAnalyze = false) {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "选择文件夹",
    });
    if (!selected || Array.isArray(selected)) return;
    const input = document.querySelector<HTMLInputElement>(`#${inputId}`);
    if (!input) return;
    input.value = selected;
    if (inputId === "project-path") {
      const validation = await invoke<{ message: string; recognizedProject: boolean }>("validate_directory_path", { path: selected });
      showToast(validation.message, !validation.recognizedProject);
      if (autoAnalyze) {
        const analysis = await invoke<ProjectAnalysis>("analyze_project", { path: selected });
        renderProjectAnalysis(analysis);
        await inspectProjectPorts(false);
      }
    }
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
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
    if (tab === "move") void loadArchivePlan();
  });
});

document.querySelectorAll<HTMLButtonElement>("[data-pick-directory]").forEach((button) => {
  button.addEventListener("click", () => {
    const target = button.dataset.pickDirectory || "";
    if (!target) return;
    void pickDirectoryInto(target, button.dataset.autoAnalyze === "true");
  });
});

document.querySelector("#refresh-all")?.addEventListener("click", () => void refreshAll(true));
document.querySelector("#load-archive-plan")?.addEventListener("click", () => void loadArchivePlan());
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
document.querySelector("#inspect-python-integrity")?.addEventListener("click", async () => {
  showToast("正在检查 Python 完整性");
  try {
    state.pythonIntegrity = await invoke<PythonIntegrityReport>("inspect_python_integrity", { pythonPath: null });
    renderPythonIntegrity();
    showToast(state.pythonIntegrity.fullyUsable ? "Python 核心组件可用" : "Python 存在核心组件缺失", !state.pythonIntegrity.fullyUsable);
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
document.querySelector("#inspect-runtime-strong")?.addEventListener("click", async () => {
  showToast("正在强验证已登记运行时");
  try {
    state.runtimeStrong = await invoke<RuntimeStrongVerificationReport>("inspect_runtime_strong_verification");
    renderRuntimeStrongVerification();
    showToast(`运行时强验证完成：${state.runtimeStrong.items.length} 项`);
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
document.querySelector("#preview-python-repair")?.addEventListener("click", async () => {
  const repairPip = document.querySelector<HTMLInputElement>("#python-repair-pip")?.checked ?? false;
  const repairPath = document.querySelector<HTMLInputElement>("#python-repair-path")?.checked ?? false;
  showToast("正在重新诊断并生成 Python 修复计划");
  try {
    state.python = await invoke<PythonAnalysis>("analyze_python_environment");
    state.pythonRepairPlan = await invoke<PythonRepairPlan>("preview_python_repair", { repairPip, repairPath });
    renderPythonAnalysis();
    renderPythonRepairPlan();
    showToast("Python 修复计划已生成；确认命令和 PATH 差异后再执行");
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
document.querySelector("#run-learning-command")?.addEventListener("click", async () => {
  const command = document.querySelector<HTMLInputElement>("#learning-command")?.value.trim() || "";
  const output = document.querySelector<HTMLElement>("#learning-output");
  try {
    const result = await invoke<CommandRunResult>("run_learning_check", { command });
    if (output) output.textContent = `退出码 ${result.returnCode} · ${result.elapsedMs} ms\n${result.output}`;
    showToast(result.success ? "只读检查完成" : "检查命令返回异常", !result.success);
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
document.querySelector("#rust-update")?.addEventListener("click", async () => {
  if (!(await askForConfirmation("rustup 将联网更新当前用户安装的 Rust 工具链，可能需要一些时间。确定继续吗？"))) return;
  void runPlatformAction("rust_update");
});
document.querySelector("#copy-cargo-mirror")?.addEventListener("click", () => {
  void copyText(`[source.crates-io]\nreplace-with = "rsproxy-sparse"\n\n[source.rsproxy-sparse]\nregistry = "sparse+https://rsproxy.cn/index/"`);
});
document.querySelector("#set-maven-mirror")?.addEventListener("click", async () => {
  const value = document.querySelector<HTMLSelectElement>("#maven-mirror")?.value || "official";
  const path = state.platforms?.mirrors.mavenSettingsPath || "%USERPROFILE%\\.m2\\settings.xml";
  if (!(await askForConfirmation(`将写入 ${path}。若文件已存在，会先创建带时间戳的备份。确定继续吗？`))) return;
  void runPlatformAction("maven_mirror", value);
});
document.querySelector("#set-gradle-mirror")?.addEventListener("click", async () => {
  const value = document.querySelector<HTMLSelectElement>("#gradle-mirror")?.value || "official";
  const path = state.platforms?.mirrors.gradleInitPath || "%USERPROFILE%\\.gradle\\init.gradle";
  if (!(await askForConfirmation(`将写入 ${path}。若文件已存在，会先创建带时间戳的备份。确定继续吗？`))) return;
  void runPlatformAction("gradle_mirror", value);
});
document.querySelector("#restore-maven-config")?.addEventListener("click", async () => {
  if (!(await askForConfirmation("将恢复最近一次 DevEnv Manager 备份的 Maven 配置，并保留当前配置备份。确定继续吗？"))) return;
  void runPlatformAction("restore_maven_config");
});
document.querySelector("#restore-gradle-config")?.addEventListener("click", async () => {
  if (!(await askForConfirmation("将恢复最近一次 DevEnv Manager 备份的 Gradle 配置，并保留当前配置备份。确定继续吗？"))) return;
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
document.querySelector("#generate-ssh-key")?.addEventListener("click", async () => {
  const email = document.querySelector<HTMLInputElement>("#git-user-email")?.value.trim() || "";
  if (!email) {
    showToast("请先填写用于 SSH Key 注释的邮箱", true);
    return;
  }
  if (!(await askForConfirmation("将在当前用户 .ssh 目录生成 id_ed25519。已有同名密钥时会自动拒绝覆盖，确定继续吗？"))) return;
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
document.querySelector("#configure-env")?.addEventListener("click", async () => {
  showToast("正在计算 DEVENV_HOME、JAVA_HOME 与 PATH 差异");
  try {
    state.environmentPreview = await invoke<EnvironmentConfigPreview>("preview_user_environment_configuration");
    renderEnvironmentPreview();
    showToast("环境配置预览已生成；确认差异后再写入");
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
document.querySelector("#inspect-env-reliability")?.addEventListener("click", async () => {
  showToast("正在读取当前进程环境、用户环境和 PATH 命中顺序");
  try {
    state.envReliability = await invoke<EnvReliabilitySnapshot>("inspect_env_reliability");
    renderEnvReliability();
    showToast(`环境可靠性检查完成：${state.envReliability.issues.length} 个问题/提示`);
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
document.querySelector("#create-java-stabilize-plan")?.addEventListener("click", async () => {
  const input = document.querySelector<HTMLInputElement>("#java-stabilize-path");
  const jdkPath = input?.value.trim() || state.envReliability?.java.javaHomeExpanded || "";
  if (!jdkPath) {
    showToast("请填写 JDK 根目录，不能填写 bin 目录", true);
    return;
  }
  showToast("正在生成 Java 稳定修复计划");
  try {
    state.envRepairPlan = await invoke<EnvRepairPlan>("create_java_stabilize_plan", { jdkPath });
    state.envRepairResult = null;
    renderEnvRepairPlan();
    showToast("计划已生成；请检查 diff、备份名和风险说明");
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
document.querySelector("#apply-env-repair-plan")?.addEventListener("click", async () => {
  const plan = state.envRepairPlan;
  if (!plan) return;
  if (!(await confirmRisk(`将写入当前用户级环境变量，并创建备份：${plan.backupName}`, plan.riskLevel))) return;
  showToast("正在应用环境修复计划并验证");
  try {
    const token = await riskOperationToken("apply_env_repair_plan", plan.planId, "high", false, plan.backupName);
    state.envRepairResult = await invoke<EnvRepairResult>("apply_env_repair_plan", { plan, confirmationToken: token.token });
    state.envRepairPlan = null;
    state.envReliability = await invoke<EnvReliabilitySnapshot>("inspect_env_reliability");
    state.envBackupRecords = await invoke<EnvBackupRecord[]>("list_env_backups");
    renderEnvRepairPlan();
    renderEnvReliability();
    renderEnvBackupRecords();
    showToast(state.envRepairResult.message, !state.envRepairResult.success);
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
document.querySelector("#export-env-reliability")?.addEventListener("click", async () => {
  try {
    const path = await invoke<string>("export_env_reliability_report", { format: "markdown" });
    showToast(`环境可靠性报告已导出：${path}`);
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
async function loadEnvironmentBackups() {
  try {
    state.environmentBackups = await invoke<EnvironmentBackupInfo[]>("list_environment_backups");
    renderEnvironmentBackups();
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
}
document.querySelector("#load-env-backups")?.addEventListener("click", () => void loadEnvironmentBackups());
document.querySelector("#load-env-backup-records")?.addEventListener("click", async () => {
  try {
    state.envBackupRecords = await invoke<EnvBackupRecord[]>("list_env_backups");
    renderEnvBackupRecords();
    showToast(`读取到 ${state.envBackupRecords.length} 个 Phase 5 环境备份`);
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
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
document.querySelector("#cleanup-path")?.addEventListener("click", async () => {
  if (!(await askForConfirmation("将删除当前用户 PATH 中真实失效或重复的条目，并先创建环境备份；受管待安装路径会保留。确定继续吗？"))) return;
  void runOperation(async () => {
    const token = await riskOperationToken("cleanup_path_entries", "cleanup-path-entries", "medium", false, "environment-backup");
    return invoke<OperationResult>("cleanup_path_entries", { confirmationToken: token.token });
  }, "正在清理真实失效和重复 PATH");
});
document.querySelector("#restore-env")?.addEventListener("click", async () => {
  if (!(await askForConfirmation("将恢复最近一次环境备份；已打开的终端和 IDE 不会自动刷新。确定继续吗？"))) return;
  void runOperation(async () => {
    const token = await riskOperationToken("restore_user_environment", "restore-user-environment-latest", "high", false, "environment-backup");
    return invoke<OperationResult>("restore_user_environment", { confirmationToken: token.token });
  }, "正在恢复用户环境变量");
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
  if (!(await askForConfirmation("将自动清理真实失效/重复 PATH，并修复 DevEnv 管理的用户级环境变量。不会安装软件、结束进程或修改系统级变量。确定继续吗？"))) return;
  showToast("正在执行安全修复并重新诊断");
  try {
    const result = await invoke<DoctorRepairResult>("repair_doctor_safe");
    state.doctor = result.report;
    renderDoctor();
    const detail = result.applied.length ? result.applied.join("\n") : "没有可自动修复的安全项目";
    const repairResult = document.querySelector<HTMLElement>("#doctor-repair-result");
    if (repairResult) {
      repairResult.innerHTML = `<article class="runtime ${result.remaining.length ? "warn" : "ok"}">
        <div><strong>安全修复结果</strong><span>${result.beforeScore} → ${result.afterScore}</span></div>
        <small>${escapeHtml(detail)}</small>
        ${result.remaining.length ? `<ul>${result.remaining.map((item) => `<li>${escapeHtml(item)}</li>`).join("")}</ul>` : "<small>没有剩余需要手动处理的自动修复项</small>"}
      </article>`;
    }
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
document.querySelector("#import-profiles")?.addEventListener("click", async () => {
  const path = document.querySelector<HTMLInputElement>("#profile-file-path")?.value.trim() || "";
  if (!path || !state.profileImportPreview) {
    showToast("请先预览并校验模板", true);
    return;
  }
  const replacements = state.profileImportPreview.profiles.filter((item) => item.willReplace).length;
  if (!(await askForConfirmation(`将导入 ${state.profileImportPreview.profiles.length} 个模板${replacements ? `，覆盖 ${replacements} 个同名模板` : ""}。确定继续吗？`))) return;
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
document.querySelector("#clear-cache")?.addEventListener("click", async () => {
  if (!(await askForConfirmation("下载缓存将逐项移入 Windows 回收站，不会删除受管运行时或配置。确定继续吗？"))) return;
  void runOperation(async () => {
    const token = await riskOperationToken("clear_download_cache", "clear-download-cache", "medium");
    return invoke<OperationResult>("clear_download_cache", { confirmationToken: token.token });
  }, "正在将下载缓存移入回收站");
});
document.querySelector("#inspect-maintenance")?.addEventListener("click", () => void inspectMaintenance());
document.querySelector("#scan-maintenance")?.addEventListener("click", () => void scanMaintenance());
function selectCleanupMode(mode: "conservative" | "recommended" | "none") {
  state.cleanupSelection.clear();
  if (mode !== "none" && state.cleanupReport) {
    state.cleanupReport.categories.forEach((category) => {
      const selectedCategory = category.id === "devenv-manager" || (mode === "recommended" && category.id === "windows-temp");
      if (selectedCategory) category.items.filter((item) => item.cleanable).forEach((item) => state.cleanupSelection.add(item.id));
    });
  }
  state.cleanupPlan = null;
  renderMaintenanceScan();
  renderCleanupPlan();
}
document.querySelector("#select-conservative")?.addEventListener("click", () => selectCleanupMode("conservative"));
document.querySelector("#select-recommended")?.addEventListener("click", () => selectCleanupMode("recommended"));
document.querySelector("#clear-cleanup-selection")?.addEventListener("click", () => selectCleanupMode("none"));
document.querySelector("#preview-cleanup-plan")?.addEventListener("click", async () => {
  if (!state.cleanupReport || !state.cleanupSelection.size) return;
  showToast("正在重新扫描并创建清理计划");
  try {
    state.cleanupPlan = await invoke<CleanupPlan>("create_cleanup_plan", { selectedItemIds: Array.from(state.cleanupSelection) });
    renderCleanupPlan();
    showToast(`计划已创建：${state.cleanupPlan.selectedItems.length} 项，预计 ${formatBytes(state.cleanupPlan.estimatedBytes)}`);
  } catch (error) {
    state.cleanupPlan = null;
    renderCleanupPlan();
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});

document.addEventListener("change", (event) => {
  const checkbox = (event.target as HTMLElement).closest<HTMLInputElement>("input[data-cleanup-item]");
  if (!checkbox) return;
  const id = checkbox.dataset.cleanupItem || "";
  if (checkbox.checked) state.cleanupSelection.add(id);
  else state.cleanupSelection.delete(id);
  state.cleanupPlan = null;
  const preview = document.querySelector<HTMLButtonElement>("#preview-cleanup-plan");
  const execute = document.querySelector<HTMLButtonElement>("#execute-cleanup-plan");
  if (preview) preview.disabled = state.cleanupSelection.size === 0;
  if (execute) execute.disabled = true;
  renderCleanupPlan();
});
document.querySelector("#execute-cleanup-plan")?.addEventListener("click", async () => {
  const plan = state.cleanupPlan;
  if (!plan) return;
  if (!(await askForConfirmation(`即将清理 ${plan.selectedItems.length} 项，预计释放 ${formatBytes(plan.estimatedBytes)}。后端会再次扫描并校验，普通文件移入回收站。确定继续吗？`))) return;
  showToast("正在重新校验并执行清理计划");
  try {
    state.cleanupResult = await invoke<CleanupResult>("clean_selected_targets", { plan });
    state.cleanupPlan = null;
    state.cleanupSelection.clear();
    renderCleanupResult();
    showToast(`清理完成：释放 ${formatBytes(state.cleanupResult.cleanedBytes)}，失败 ${state.cleanupResult.failedItems} 项`, state.cleanupResult.failedItems > 0);
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
document.querySelector("#inspect-desktop")?.addEventListener("click", async () => {
  showToast("正在只读分析桌面文件类型与占用");
  try {
    state.desktopUsage = await invoke<FolderUsageReport>("inspect_desktop");
    renderFolderUsage("#desktop-usage", state.desktopUsage, "desktop-usage");
    showToast(`桌面分析完成：${formatBytes(state.desktopUsage.totalBytes)}`);
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
document.querySelector("#inspect-downloads")?.addEventListener("click", async () => {
  showToast("正在只读分类下载目录");
  try {
    state.downloadsUsage = await invoke<FolderUsageReport>("inspect_downloads");
    renderFolderUsage("#downloads-usage", state.downloadsUsage, "downloads-usage");
    showToast(`下载目录分析完成：${formatBytes(state.downloadsUsage.totalBytes)}`);
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});

function setScanBusy(scanSelector: string, cancelSelector: string, busy: boolean) {
  const scanButton = document.querySelector<HTMLButtonElement>(scanSelector);
  const cancelButton = document.querySelector<HTMLButtonElement>(cancelSelector);
  if (scanButton) scanButton.disabled = busy;
  if (cancelButton) cancelButton.disabled = !busy;
}

document.querySelector("#scan-large-files")?.addEventListener("click", async () => {
  const root = document.querySelector<HTMLInputElement>("#large-file-root")?.value.trim() || "";
  const minSizeMb = Number(document.querySelector<HTMLInputElement>("#large-file-min")?.value || "100");
  showToast("正在只读扫描大文件；不会读取文件内容");
  setScanBusy("#scan-large-files", "#cancel-large-scan", true);
  try {
    state.largeFiles = await invoke<LargeFileItem[]>("scan_large_files", { root, minSizeMb, limit: 100 });
    renderLargeFiles();
    showToast(`大文件扫描完成：${state.largeFiles.length} 项`);
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  } finally {
    setScanBusy("#scan-large-files", "#cancel-large-scan", false);
  }
});
document.querySelector("#scan-duplicates")?.addEventListener("click", async () => {
  const root = document.querySelector<HTMLInputElement>("#duplicate-root")?.value.trim() || "";
  const minSizeMb = Number(document.querySelector<HTMLInputElement>("#duplicate-min")?.value || "10");
  if (!(await askForConfirmation(`将只在“${root || "用户目录"}”内对 ${minSizeMb} MB 以上、大小相同的候选文件计算 SHA256。不会上传或删除文件，确定继续吗？`))) return;
  showToast("正在按大小分组并计算重复候选 SHA256");
  setScanBusy("#scan-duplicates", "#cancel-duplicate-scan", true);
  try {
    state.duplicateGroups = await invoke<DuplicateGroup[]>("scan_duplicate_large_files", { root, minSizeMb });
    renderDuplicates();
    showToast(`重复文件扫描完成：${state.duplicateGroups.length} 组`);
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  } finally {
    setScanBusy("#scan-duplicates", "#cancel-duplicate-scan", false);
  }
});
document.querySelector("#cancel-large-scan")?.addEventListener("click", async () => {
  const result = await invoke<OperationResult>("cancel_maintenance_scan");
  showToast(result.message);
});
document.querySelector("#cancel-duplicate-scan")?.addEventListener("click", async () => {
  const result = await invoke<OperationResult>("cancel_maintenance_scan");
  showToast(result.message);
});
document.querySelector("#inspect-app-usage")?.addEventListener("click", async () => {
  showToast("正在只读统计常见应用、游戏库与已安装软件");
  try {
    state.appUsage = await invoke<AppUsageReport>("inspect_app_usage");
    renderAppUsage();
    showToast("软件与常见应用分析完成");
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
document.querySelector("#preview-move-plan")?.addEventListener("click", async () => {
  const source = document.querySelector<HTMLInputElement>("#move-source")?.value.trim() || "";
  const targetDrive = document.querySelector<HTMLInputElement>("#move-target-drive")?.value.trim() || "D:";
  const mode = document.querySelector<HTMLSelectElement>("#move-mode")?.value || "archive_only";
  if (!source) {
    showToast("请先填写源目录", true);
    return;
  }
  showToast("正在生成空间搬家计划");
  try {
    state.movePlan = await invoke<MovePlan>("create_move_plan", { source, targetDrive, mode });
    state.moveResult = null;
    renderMovePlan();
    showToast(`搬家计划已生成：${formatBytes(state.movePlan.estimatedBytes)}`);
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
document.querySelector("#preview-desktop-archive")?.addEventListener("click", async () => {
  const targetDrive = document.querySelector<HTMLInputElement>("#move-target-drive")?.value.trim() || "D:";
  showToast("正在生成桌面归档计划");
  try {
    state.movePlan = await invoke<MovePlan>("create_desktop_archive_plan", { targetDrive });
    state.moveResult = null;
    renderMovePlan();
    showToast("桌面归档计划已生成");
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
document.querySelector("#preview-downloads-archive")?.addEventListener("click", async () => {
  const targetDrive = document.querySelector<HTMLInputElement>("#move-target-drive")?.value.trim() || "D:";
  showToast("正在生成下载目录归档计划");
  try {
    state.movePlan = await invoke<MovePlan>("create_downloads_archive_plan", { targetDrive });
    state.moveResult = null;
    renderMovePlan();
    showToast("下载归档计划已生成");
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
document.querySelector("#execute-move-plan")?.addEventListener("click", async () => {
  const plan = state.movePlan;
  if (!plan) return;
  if (!(await askForConfirmation(`将执行 ${plan.mode}：\n${plan.source}\n→ ${plan.target}\n\n执行前请关闭相关程序。确定继续吗？`))) return;
  showToast("正在执行空间搬家/归档计划");
  try {
    const command = plan.source.toLowerCase().includes("\\desktop") && plan.mode === "archive_only"
      ? "execute_desktop_archive_plan"
      : plan.source.toLowerCase().includes("\\downloads") && plan.mode === "archive_only"
        ? "execute_downloads_archive_plan"
        : "execute_move_plan";
    const token = await riskOperationToken("execute_move_plan", plan.planId, "high", false, "move-plan-preview");
    state.moveResult = await invoke<MoveResult>(command, { plan, confirmationToken: token.token });
    renderMovePlan();
    await loadRollbackRecords();
    showToast(`执行完成：${formatBytes(state.moveResult.movedBytes)}，失败 ${state.moveResult.failures.length} 项`, state.moveResult.failures.length > 0);
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
document.querySelector("#load-rollback-records")?.addEventListener("click", () => void loadRollbackRecords());
document.querySelector("#inspect-partition-layout")?.addEventListener("click", async () => {
  showToast("正在只读检测磁盘分区布局");
  try {
    state.partitionLayout = await invoke<PartitionLayoutReport>("inspect_partition_layout");
    renderPartitionLayout();
    showToast("分区检测完成");
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
document.querySelector("#create-expansion-plan")?.addEventListener("click", async () => {
  showToast("正在生成 C 盘扩容安全计划");
  try {
    state.expansionPlan = await invoke<ExpansionPlan>("create_c_drive_expansion_plan");
    state.expansionResult = null;
    renderExpansionPlan();
    showToast(state.expansionPlan.canExecute ? "扩容计划可执行，但仍需三次确认" : "当前只生成说明计划，不允许执行");
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
document.querySelector("#execute-expansion-plan")?.addEventListener("click", async () => {
  const plan = state.expansionPlan;
  if (!plan?.canExecute) return;
  const prompts = [
    "扩容会修改磁盘分区表。请确认已经备份重要数据，输入 YES 继续。",
    `计划模式：${plan.mode}，需要管理员权限。再次输入 YES 继续。`,
    "最后确认：执行期间不要断电，不要关闭程序。输入 YES 执行。",
  ];
  for (const prompt of prompts) {
    if (!(await askForConfirmation(prompt, { title: "确认磁盘扩容操作", danger: true, requiredText: "YES" }))) {
      showToast("已取消扩容执行");
      return;
    }
  }
  showToast("正在执行 C 盘扩容计划");
  try {
    const token = await riskOperationToken("execute_expansion_plan", plan.planId, "critical", true, "manual-backup-confirmed");
    state.expansionResult = await invoke<ExpansionResult>("execute_c_drive_expansion", { plan, confirmationToken: token.token });
    renderExpansionPlan();
    showToast(state.expansionResult.success ? "扩容执行完成" : "扩容未成功，请查看报告", !state.expansionResult.success);
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
document.querySelector("#check-updates")?.addEventListener("click", async () => {
  await checkUpdates();
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
document.querySelector("#inspect-mysql-repair")?.addEventListener("click", async () => {
  showToast("正在只读检查 MySQL 服务、配置与 Data 健康状态");
  try {
    state.mysqlRepair = await invoke<MySqlRepairReport>("inspect_mysql_repair");
    state.mysqlPlan = null;
    renderMySqlRepair();
    renderMySqlPlan();
    showToast(`MySQL 诊断完成：发现 ${state.mysqlRepair.candidates.length} 个候选`);
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});
document.querySelector("#open-docker-desktop")?.addEventListener("click", () => {
  void runOperation(() => invoke<OperationResult>("open_docker_desktop"), "正在启动 Docker Desktop");
});
document.querySelector("#self-uninstall")?.addEventListener("click", async () => {
  const ok = await askForConfirmation("这会启动 DevEnv Manager 的卸载程序并关闭当前程序。确定继续吗？");
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
      confirmed = await askForConfirmation(`${assessment.reason}\n\n命令：${command}\n\n确定继续吗？`, {
        title: "确认运行白名单命令",
        danger: assessment.risk === "high" || assessment.risk === "critical",
      });
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
document.querySelector("#preview-project-config")?.addEventListener("click", async () => {
  const input = document.querySelector<HTMLInputElement>("#project-path");
  if (!input) return;
  showToast("正在生成 VS Code / IDEA 配置预览");
  try {
    state.projectConfigPreview = await invoke<ProjectConfigPreview>("preview_project_configuration", { projectPath: input.value });
    renderProjectConfigPreview();
    showToast("配置预览已生成；确认内容后再应用");
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});

document.querySelector("#inspect-idea-project")?.addEventListener("click", async () => {
  const path = document.querySelector<HTMLInputElement>("#project-path")?.value.trim() || "";
  showToast("正在只读读取 IDEA 配置");
  try {
    state.ideaProject = await invoke<IdeaProjectReport>("inspect_idea_project", { path });
    renderIdeaProject();
    showToast(state.ideaProject.detected ? "IDEA 配置分析完成" : "未发现 IDEA 配置");
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});

document.querySelector("#verify-nacos-java")?.addEventListener("click", async () => {
  const root = document.querySelector<HTMLInputElement>("#project-path")?.value.trim() || "";
  showToast("正在验证 Nacos Java 环境");
  try {
    state.javaConsumer = await invoke<JavaConsumerReport>("verify_java_consumer_environment", { consumer: "Nacos", root });
    renderJavaConsumer();
    showToast(state.javaConsumer.usable ? "Nacos 可读取当前 Java 环境" : "Nacos Java 环境需要关注", !state.javaConsumer.usable);
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
});

document.querySelector("#verify-nexus-java")?.addEventListener("click", async () => {
  const root = document.querySelector<HTMLInputElement>("#project-path")?.value.trim() || "";
  showToast("正在验证 Nexus Java 环境");
  try {
    state.javaConsumer = await invoke<JavaConsumerReport>("verify_nexus_java_environment", { root });
    renderJavaConsumer();
    showToast(state.javaConsumer.usable ? "Nexus 可读取当前 Java 环境" : "Nexus Java 环境需要关注", !state.javaConsumer.usable);
  } catch (error) {
    showToast(error instanceof Error ? error.message : String(error), true);
  }
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
  paginationState.set("ports", 1);
  renderPorts();
});

function setPortQuickFilter(filter: string) {
  portState.quickFilter = filter || "all";
  document.querySelectorAll<HTMLElement>("[data-port-filter]").forEach((item) => {
    item.classList.toggle("active", item.dataset.portFilter === portState.quickFilter);
  });
  paginationState.set("ports", 1);
  renderPorts();
}

document.querySelectorAll("#port-quick-filters, #port-summary").forEach((element) => {
  element.addEventListener("click", (event) => {
    const button = (event.target as HTMLElement).closest<HTMLButtonElement>("button[data-port-filter]");
    if (!button) return;
    setPortQuickFilter(button.dataset.portFilter || "all");
  });
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

document.addEventListener("click", async (event) => {
  const button = (event.target as HTMLElement).closest<HTMLButtonElement>(
    "button[data-action], button[data-toolchain-action], button[data-python-tool], button[data-page-key], button[data-dev-cache], button[data-chsrc-action], button[data-cleanup-report-action], button[data-restore-env-backup], button[data-mysql-action], #apply-project-config, #apply-environment-preview, #apply-python-repair, #create-managed-python-pip-plan, #execute-mysql-plan, #accept-safety-disclaimer",
  );
  if (!button) return;
  const mysqlAction = button.dataset.mysqlAction;
  if (mysqlAction) {
    const candidateId = button.dataset.candidate || "";
    void invoke<MySqlRepairPlan>("create_mysql_repair_plan", { candidateId, action: mysqlAction })
      .then((plan) => {
        state.mysqlPlan = plan;
        renderMySqlPlan();
        showToast("MySQL 一次性计划已生成；请逐项核对");
      })
      .catch((error) => showToast(error instanceof Error ? error.message : String(error), true));
    return;
  }
  if (button.id === "apply-python-repair") {
    const plan = state.pythonRepairPlan;
    if (!plan) return;
    if (!(await askForConfirmation(`将执行 ${plan.actions.length} 项 Python 修复，并先保存用户环境备份。pip 升级可能联网，确定继续吗？`))) return;
    void runOperation(() => invoke<OperationResult>("apply_python_repair", { planId: plan.planId }), "正在执行并验证 Python 修复").then(async () => {
      state.pythonRepairPlan = null;
      state.python = await invoke<PythonAnalysis>("analyze_python_environment");
      renderPythonAnalysis();
      renderPythonRepairPlan();
    });
    return;
  }
  if (button.id === "execute-mysql-plan") {
    const plan = state.mysqlPlan;
    if (!plan) return;
    const backupDestination = document.querySelector<HTMLInputElement>("#mysql-backup-destination")?.value.trim() || null;
    const guideOnly = plan.action === "reset_root_guide" || plan.action === "dump_guide";
    if (!guideOnly && !(await askForConfirmation(`将执行 MySQL 计划“${plan.title}”。程序会重新诊断路径和状态；失败不会绕过保护规则。确定继续吗？`, {
      title: "确认 MySQL 修复计划",
      danger: true,
    }))) return;
    void (async () => {
      showToast(guideOnly ? "正在生成安全向导" : "正在执行 MySQL 修复计划");
      try {
        let confirmationToken: string | null = null;
        if (!guideOnly) {
          const guard = await invoke<MySqlExecutionGuard>("mysql_pending_execution_guard", { planId: plan.planId });
          if (guard.riskLevel === "critical") {
            if (!(await askForConfirmation("第一次确认：MySQL 系统库修复前必须已完成完整 Data 备份。"))) return;
            if (!(await askForConfirmation("第二次确认：我理解该操作可能影响数据库服务启动和业务库恢复。"))) return;
            if (!(await askForConfirmation("第三次确认：请输入指定文本后才允许继续。", {
              title: "最终确认 MySQL 高危修复",
              danger: true,
              requiredText: "我已知晓 MySQL 修复风险并确认执行",
            }))) return;
          }
          const token = await createBackendConfirmation(
            guard.actionId,
            guard.planId,
            guard.riskLevel,
            guard.planFingerprint,
            guard.riskLevel === "critical",
            guard.backupReceipt || null,
            "execute_mysql_repair_plan",
          );
          confirmationToken = token.token;
        }
        const result = await invoke<OperationResult>("execute_mysql_repair_plan", { planId: plan.planId, backupDestination, confirmationToken });
        if (guideOnly) {
          const output = document.querySelector<HTMLElement>("#local-service-logs");
          if (output) output.textContent = result.message;
        }
        showToast(result.message);
        state.mysqlPlan = null;
        state.mysqlRepair = await invoke<MySqlRepairReport>("inspect_mysql_repair");
        renderMySqlPlan();
        renderMySqlRepair();
      } catch (error) {
        showToast(error instanceof Error ? error.message : String(error), true);
      }
    })();
    return;
  }
  const pageKey = button.dataset.pageKey;
  if (pageKey) {
    paginationState.set(pageKey, Number(button.dataset.page || "1"));
    if (pageKey === "ports") renderPorts();
    else if (pageKey === "port-history") renderPortHistory();
    else if (pageKey === "runtime-list" || pageKey.startsWith("managed-")) renderRuntimes();
    else if (pageKey === "runtime-strong") renderRuntimeStrongVerification();
    else if (pageKey === "path-warnings") renderEnv();
    else if (pageKey === "env-health") renderHealth();
    else if (pageKey === "profiles") renderProfiles();
    else if (pageKey === "agent-traces") renderAgentTraces();
    else if (pageKey === "wsl-items") renderSystemPlatforms();
    else if (pageKey === "local-services") renderLocalServices();
    else if (pageKey === "download-cache") renderCache();
    else if (pageKey === "project-ports") renderProjectPortConfigs();
    else if (pageKey.startsWith("project-") && state.project) renderProjectAnalysis(state.project);
    else if (pageKey === "environment-backups") renderEnvironmentBackups();
    else if (pageKey.startsWith("desktop-usage")) renderFolderUsage("#desktop-usage", state.desktopUsage, "desktop-usage");
    else if (pageKey.startsWith("downloads-usage")) renderFolderUsage("#downloads-usage", state.downloadsUsage, "downloads-usage");
    else if (pageKey === "large-files") renderLargeFiles();
    else if (pageKey === "archive-plan") renderArchivePlan();
    else if (pageKey === "rollback-records") renderRollbackRecords();
    else if (pageKey === "duplicate-groups" || pageKey.startsWith("duplicate-")) renderDuplicates();
    else if (pageKey === "app-usage" || pageKey === "installed-software") renderAppUsage();
    else if (pageKey.startsWith("cleanup-")) {
      renderMaintenanceScan();
      renderCleanupPlan();
    } else if (pageKey.startsWith("dotnet-") || pageKey === "rust-toolchains") renderPlatforms();
    return;
  }
  const devCache = button.dataset.devCache;
  if (devCache) {
    if (!(await askForConfirmation(`将调用 ${button.title || button.textContent || devCache}。该命令会清除可重新生成的开发缓存，确定继续吗？`))) return;
    void runOperation(async () => {
      const token = await riskOperationToken("clean_dev_cache", `tool-${devCache.trim().toLowerCase()}`, "medium");
      return invoke<OperationResult>("clean_dev_cache", { tool: devCache, confirmationToken: token.token });
    }, `正在使用 ${devCache} 官方命令清理缓存`).then(() => void scanMaintenance());
    return;
  }
  const chsrcAction = button.dataset.chsrcAction;
  if (chsrcAction) {
    const target = document.querySelector<HTMLSelectElement>("#chsrc-target")?.value || "node";
    const source = document.querySelector<HTMLInputElement>("#chsrc-source")?.value.trim() || null;
    const changing = ["auto", "set", "reset"].includes(chsrcAction);
    if (changing && !(await askForConfirmation(`将调用官方 chsrc 对 ${target} 执行 ${chsrcAction}，可能修改当前用户或工具配置。确定继续吗？`, {
      title: "确认 chsrc 配置操作",
      danger: true,
    }))) return;
    void (async () => {
      try {
        const result = await invoke<OperationResult>("run_chsrc_action", { action: chsrcAction, target, source });
        const output = document.querySelector<HTMLElement>("#chsrc-output");
        if (output) output.textContent = result.message;
        showToast("chsrc 操作完成");
      } catch (error) {
        showToast(error instanceof Error ? error.message : String(error), true);
      }
    })();
    return;
  }
  if (button.dataset.cleanupReportAction === "copy") {
    if (state.cleanupResult) void copyText(state.cleanupResult.reportMarkdown);
    return;
  }
  if (button.dataset.cleanupReportAction === "json") {
    void invoke<string>("export_cleanup_report", { format: "json" })
      .then((path) => showToast(`JSON 报告已导出：${path}`))
      .catch((error) => showToast(error instanceof Error ? error.message : String(error), true));
    return;
  }
  if (button.id === "create-managed-python-pip-plan") {
    const report = state.pythonIntegrity;
    if (!report) return;
    void (async () => {
      showToast("正在生成受管 Python pip 修复计划");
      try {
        state.pythonRepairPlan = await invoke<PythonRepairPlan>("create_managed_python_pip_repair_plan", { pythonPath: report.pythonPath });
        renderPythonRepairPlan();
        showToast("pip 修复计划已生成；请核对后执行");
      } catch (error) {
        showToast(error instanceof Error ? error.message : String(error), true);
      }
    })();
    return;
  }
  if (button.id === "accept-safety-disclaimer") {
    void runOperation(() => invoke<OperationResult>("accept_safety_disclaimer"), "正在记录安全说明已读状态").then(async () => {
      if (state.config) {
        state.config.settings.safetyDisclaimerAccepted = true;
        state.config.settings.safetyDisclaimerVersion = SAFETY_DISCLAIMER_VERSION;
        state.config.settings.safetyDisclaimerAcceptedAt = new Date().toISOString();
      }
      renderSafetyDisclaimer();
      renderSafetyGate();
    });
    return;
  }
  if (button.dataset.action === "restore-env-record") {
    const backupName = button.dataset.backupName || "";
    if (!(await confirmRisk(`将恢复用户级环境变量备份：${backupName}\n恢复前会先备份当前状态。`, "medium"))) return;
    void invoke<EnvRepairResult>("restore_env_backup", { backupName })
      .then(async (result) => {
        state.envRepairResult = result;
        state.envReliability = await invoke<EnvReliabilitySnapshot>("inspect_env_reliability");
        state.envBackupRecords = await invoke<EnvBackupRecord[]>("list_env_backups");
        renderEnvReliability();
        renderEnvRepairPlan();
        renderEnvBackupRecords();
        showToast(result.message, !result.success);
      })
      .catch((error) => showToast(error instanceof Error ? error.message : String(error), true));
    return;
  }
  if (button.dataset.action === "rollback-move") {
    const rollbackId = button.dataset.rollbackId || "";
    if (!(await askForConfirmation(`将执行回滚 ${rollbackId}：删除 Junction 并恢复备份目录（如存在）。确定继续吗？`))) return;
    void runOperation(async () => {
      const token = await riskOperationToken("rollback_move", rollbackId, "high");
      return invoke<OperationResult>("rollback_move", { rollbackId, confirmationToken: token.token });
    }, "正在执行空间搬家回滚").then(() => void loadRollbackRecords());
    return;
  }
  if (button.id === "apply-project-config") {
    const preview = state.projectConfigPreview;
    if (!preview) return;
    preview.files.forEach((file, index) => {
      file.enabled = document.querySelector<HTMLInputElement>(`[data-project-file-enabled="${index}"]`)?.checked ?? false;
      file.content = document.querySelector<HTMLTextAreaElement>(`[data-project-file-content="${index}"]`)?.value || "";
    });
    const switches: CurrentVersions = {};
    document.querySelectorAll<HTMLSelectElement>("[data-project-switch]").forEach((select) => {
      if (select.value) switches[select.dataset.projectSwitch as keyof CurrentVersions] = select.value;
    });
    const enabled = preview.files.filter((file) => file.enabled).length;
    const switchCount = Object.keys(switches).length;
    if (!(await askForConfirmation(`将写入 ${enabled} 个固定项目配置文件并切换 ${switchCount} 个运行时。已有文件和切换前环境都会备份，确定继续吗？`))) return;
    void runOperation(
      async () => {
        const request = { projectPath: preview.projectPath, files: preview.files, switches };
        const token = await riskOperationToken("apply_project_configuration", projectConfigurationPlanId(preview.projectPath, enabled, switchCount), "high", false, "project-backup");
        return invoke<OperationResult>("apply_project_configuration", { request, confirmationToken: token.token });
      },
      "正在备份并应用项目配置",
    );
    return;
  }
  if (button.id === "apply-environment-preview") {
    const preview = state.environmentPreview;
    if (!preview) return;
    if (!(await askForConfirmation(`将按预览写入 ${preview.changes.length} 组当前用户环境配置，并先保存 ${preview.backupName}。确定继续吗？`))) return;
    void runOperation(
      async () => {
        const token = await riskOperationToken("apply_user_environment_configuration", preview.previewId, "high", false, preview.backupName);
        return invoke<OperationResult>("apply_user_environment_configuration", { previewId: preview.previewId, confirmationToken: token.token });
      },
      "正在备份、写入并回读验证用户环境变量",
    ).then(async () => {
      state.environmentPreview = null;
      renderEnvironmentPreview();
      await loadEnvironmentBackups();
      await refreshAll(false);
    });
    return;
  }
  const restoreBackup = button.dataset.restoreEnvBackup;
  if (restoreBackup) {
    if (!(await askForConfirmation(`将恢复环境备份 ${restoreBackup}；恢复前会再保存当前状态。确定继续吗？`))) return;
    void runOperation(
      () => invoke<OperationResult>("restore_environment_backup", { fileName: restoreBackup }),
      "正在恢复指定环境备份",
    ).then(async () => {
      await loadEnvironmentBackups();
      await refreshAll(false);
    });
    return;
  }
  const action = button.dataset.action;
  if (action === "open-learning") {
    activateView("learning");
    return;
  }
  if (action === "archive-add") {
    void invoke<OperationResult>("add_archive_plan_item", { path: button.dataset.path || "", source: button.dataset.source || "空间分析" })
      .then(async (result) => {
        showToast(result.message);
        await loadArchivePlan();
      })
      .catch((error) => showToast(error instanceof Error ? error.message : String(error), true));
    return;
  }
  if (action === "archive-remove") {
    void invoke<OperationResult>("remove_archive_plan_item", { id: button.dataset.archiveId || "" })
      .then(async (result) => {
        showToast(result.message);
        await loadArchivePlan();
      })
      .catch((error) => showToast(error instanceof Error ? error.message : String(error), true));
    return;
  }
  if (action === "open-analysis-path") {
    void invoke<OperationResult>("open_analysis_path", { path: button.dataset.path || "" })
      .then((result) => showToast(result.message))
      .catch((error) => showToast(error instanceof Error ? error.message : String(error), true));
    return;
  }
  if (action === "rescan-folder") {
    const target = button.dataset.target || "";
    if (target === "desktop") {
      showToast("正在重新扫描桌面");
      void invoke<FolderUsageReport>("inspect_desktop")
        .then((report) => {
          state.desktopUsage = report;
          renderFolderUsage("#desktop-usage", report, "desktop-usage");
          showToast("桌面明细已刷新");
        })
        .catch((error) => showToast(error instanceof Error ? error.message : String(error), true));
    } else if (target === "downloads") {
      showToast("正在重新扫描下载目录");
      void invoke<FolderUsageReport>("inspect_downloads")
        .then((report) => {
          state.downloadsUsage = report;
          renderFolderUsage("#downloads-usage", report, "downloads-usage");
          showToast("下载目录明细已刷新");
        })
        .catch((error) => showToast(error instanceof Error ? error.message : String(error), true));
    } else if (target === "large-files") {
      document.querySelector<HTMLButtonElement>("#scan-large-files")?.click();
    }
    return;
  }
  if (action === "open-apps-features") {
    void invoke<OperationResult>("open_apps_features")
      .then((result) => showToast(result.message))
      .catch((error) => showToast(error instanceof Error ? error.message : String(error), true));
    return;
  }
  if (action === "open-python-alias-settings") {
    void invoke<OperationResult>("open_python_alias_settings")
      .then((result) => showToast(result.message))
      .catch((error) => showToast(error instanceof Error ? error.message : String(error), true));
    return;
  }
  if (action === "export-python-diagnostic") {
    void invoke<OperationResult>("export_python_diagnostic_report")
      .then((result) => showToast(result.message))
      .catch((error) => showToast(error instanceof Error ? error.message : String(error), true));
    return;
  }
  if (action === "open-app-config-dir") {
    void invoke<OperationResult>("open_app_config_dir")
      .then((result) => showToast(result.message))
      .catch((error) => showToast(error instanceof Error ? error.message : String(error), true));
    return;
  }
  if (action === "reset-ui-config") {
    void invoke<OperationResult>("reset_ui_config")
      .then((result) => {
        showToast(result.message);
        state.safeMode = false;
        state.fatalError = "";
        renderFatalError();
        return refreshAll(true);
      })
      .catch((error) => showToast(error instanceof Error ? error.message : String(error), true));
    return;
  }
  if (action === "retry-app-init") {
    state.safeMode = false;
    state.fatalError = "";
    renderFatalError();
    void refreshAll(true).catch((error) => enterSafeMode(error, "重试初始化失败"));
    return;
  }
  if (action === "copy-diagnostics") {
    void copyText(`DevEnv Manager safe mode\n${state.fatalError}`);
    return;
  }
  if (action === "dismiss-safe-mode-banner") {
    state.safeModeNoticeCollapsed = true;
    renderFatalError();
    return;
  }
  if (action === "hide-toast") {
    hideToast();
    return;
  }
  if (action === "copy-safety-disclaimer") {
    void copyText(state.safetyDisclaimer || "DevEnv Manager safety disclaimer");
    return;
  }
  if (action === "refresh-platforms") {
    void inspectPlatforms();
    return;
  }
  if (action === "check-updates") {
    void checkUpdates();
    return;
  }
  if (action === "verify-external-jdk") {
    const jdkPath = button.dataset.jdkPath || "";
    showToast("正在只读验证 JDK 的 java、javac 和 jar");
    void invoke<ValidationCheck[]>("verify_external_jdk", { jdkPath })
      .then((checks) => {
        state.externalJdkChecks[jdkPath] = checks;
        renderJavaEnvironment();
        showToast(checks.every((item) => item.success) ? "外部 JDK 验证通过" : "外部 JDK 验证未完全通过", !checks.every((item) => item.success));
      })
      .catch((error) => showToast(error instanceof Error ? error.message : String(error), true));
    return;
  }
  if (action === "set-java-home-candidate") {
    const jdkPath = button.dataset.jdkPath || "";
    const input = document.querySelector<HTMLInputElement>("#java-stabilize-path");
    if (input) input.value = jdkPath;
    activateView("environment");
    document.querySelector<HTMLButtonElement>("#create-java-stabilize-plan")?.click();
    return;
  }
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
    if (!(await askForConfirmation(`${labels[platformAction] || "执行平台操作"}。需要管理员权限时 Windows 会显示 UAC，确定继续吗？`))) return;
    void runOperation(
      async () => {
        const planId = `${platformAction}:${value || ""}`;
        const token = await riskOperationToken("manage_system_platform", planId, "high");
        return invoke<OperationResult>("manage_system_platform", { action: platformAction, value: value || null, confirmationToken: token.token });
      },
      `正在${labels[platformAction] || "执行平台操作"}`,
    ).then(async () => {
      state.systemPlatforms = await invoke<SystemPlatformReport>("inspect_system_platforms");
      renderSystemPlatforms();
    });
  }
  if (action === "download-update") {
    void (async () => {
      if (!(await askForConfirmation("将从 GitHub Releases 下载新版安装包，并使用发布清单中的 SHA256 校验。确定继续吗？"))) return;
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
    if (!(await askForConfirmation("将启动已校验的安装器并退出当前程序。请保存正在进行的工作，确定继续吗？"))) return;
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
    if (!(await askForConfirmation(`将备份 ${config.file}，并把端口 ${config.currentPort} 修改为 ${newPort}。确定继续吗？`))) return;
    void runOperation(
      async () => {
        const token = await riskOperationToken("update_project_port", `${path}:${configId}:${newPort}`, "medium", false, "project-port-backup");
        return invoke<OperationResult>("update_project_port", { path, configId, newPort, confirmationToken: token.token });
      },
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
    if (!(await askForConfirmation(`将${actionLabel} Windows 服务 ${serviceName}。数据库连接可能短暂中断，确定继续吗？`))) return;
    void runOperation(
      async () => {
        const token = await riskOperationToken("manage_local_service", `${serviceName}:${serviceAction}`, "high");
        return invoke<OperationResult>("manage_local_service", { serviceName, action: serviceAction, confirmationToken: token.token });
      },
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
    const ok = await askForConfirmation(`将停止 Windows 服务 ${serviceName}（端口 ${port}）。这会中断当前数据库连接，确定继续吗？`);
    if (!ok) return;
    void runOperation(
      async () => {
        const token = await riskOperationToken("stop_local_service", `${port}:${serviceName}`, "high");
        return invoke<OperationResult>("stop_local_service", { port, serviceName, confirmationToken: token.token });
      },
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
    const ok = longRunning || (await askForConfirmation(`将运行：${command}\n\n工作目录：${input.value}\n\n确定继续吗？`));
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
    if (!(await askForConfirmation(`将修改当前用户的 JDK 生效链：\n\nJAVA_HOME：${currentHome}\n→ ${targetHome}\n\n受管 PATH 中的 JDK 会保持在首位；切换后将自动验证 java、javac、Maven 与 Gradle。确定继续吗？`))) return;
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
        if (!(await askForConfirmation(message))) return;
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

window.addEventListener("error", (event) => {
  enterSafeMode(event.error || event.message, "前端运行时错误");
});

window.addEventListener("unhandledrejection", (event) => {
  enterSafeMode(event.reason, "未处理的异步错误");
});

window.addEventListener(
  "keydown",
  (event) => {
    const gate = document.querySelector<HTMLElement>("#safety-gate");
    if (gate && !gate.hidden && event.key === "Escape") {
      event.preventDefault();
      event.stopImmediatePropagation();
    }
  },
  true,
);

void listen<TaskProgress>("task-progress", (event) => renderProgress(event.payload));
void refreshAll(false).then(() => {
  if (state.safeMode) return;
  window.setTimeout(() => void refreshRuntimeAndPorts(true), 350);
  window.setInterval(async () => {
    if (state.safeMode) return;
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
}).catch((error) => enterSafeMode(error, "初始化失败"));
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

