import type { FeatureRiskInfo } from "../../types";

type GuideDefinition = {
  title: string;
  intro: string;
  steps: string[];
  readonly: string[];
  writes: string[];
  safety: string[];
};

const VIEW_FEATURE_MAP: Record<string, string> = {
  overview: "overview",
  doctor: "doctor",
  ports: "ports",
  runtimes: "runtime-switch",
  environment: "environment",
  project: "project",
  toolchains: "toolchains",
  platforms: "toolchains",
  learning: "learning",
  maintenance: "cleanup",
  toolbox: "command-panel",
};

const VIEW_GUIDES: Record<string, GuideDefinition> = {
  overview: {
    title: "总览",
    intro: "总览用于快速判断这台 Windows 开发机当前是否健康：它聚合当前生效运行时、PATH 风险、端口数量、版本更新状态和关键提醒。适合刚打开软件时先看一眼，再决定去环境医生、端口、项目或空间分析页面继续排查。",
    steps: ["先点“刷新”读取最新快照。", "如果看到 PATH、JAVA_HOME、端口或更新异常，再跳转到对应页面。", "需要给别人描述问题时，优先复制总览和环境医生报告。"],
    readonly: ["读取当前配置、受管运行时清单、版本更新清单和本机状态。", "不会安装软件、修改环境变量、停止进程或清理文件。"],
    writes: ["总览页本身没有写入动作；更新下载/安装会跳到工具箱更新流程处理。"],
    safety: ["如果检查失败，不影响其它页面继续使用。", "更新检查失败通常是网络或 GitHub 访问问题，可以复制错误后稍后重试。"],
  },
  doctor: {
    title: "环境医生",
    intro: "环境医生把 PATH、JAVA_HOME、Python/pip、端口、缓存和常见配置问题整理成可解释的证据。适合“不知道哪里坏了”的场景，用它先定位原因，再决定是否执行安全修复。",
    steps: ["先点“一键诊断”。", "逐条查看警告和建议，优先处理影响当前项目启动的问题。", "执行修复前先看页面里的 diff、备份名和风险等级。"],
    readonly: ["诊断、导出报告、复制建议和网络/端口检查是只读。", "报告会脱敏本机用户名、令牌和敏感路径片段。"],
    writes: ["安全修复可能写入当前用户 PATH 或受管环境变量。", "PATH 清理会移除重复、失效或旧 DevEnv 受管残留项。"],
    safety: ["修改类动作需要后端 confirmation token，并在执行前建立可恢复记录。", "失败后先重新诊断，再从备份列表恢复，不要反复点击同一个修复按钮。"],
  },
  ports: {
    title: "端口管理",
    intro: "端口管理用于识别本机监听端口、进程身份、冲突证据、父进程、Windows 服务和历史记录。适合排查 8080、3306、5173、6379 等端口被占用或误判的问题。",
    steps: ["先点“扫描”。", "点击冲突徽标或详情按钮查看证据来源。", "用搜索、排序和快捷筛选缩小到数据库、Web、桌面应用或未知进程。"],
    readonly: ["扫描、详情、复制 curl/连接命令、打开进程位置都是只读。", "端口身份不会只凭端口号下结论，会结合进程名、路径、命令行、服务名和冲突证据。"],
    writes: ["安全结束进程、停止服务会改变运行状态。", "系统关键进程、PID 过低或高风险进程不会显示结束入口。"],
    safety: ["结束进程和停止服务都需要后端 token。", "如果端口属于数据库或系统服务，优先用服务管理入口停止，不建议直接杀进程。"],
  },
  runtimes: {
    title: "运行时",
    intro: "运行时页面用于发现、验证和切换 JDK、Node.js、Python、Maven、Gradle、Go 等开发工具。它区分 DevEnv 受管版本、系统安装版本、IDE 自带版本和外部手动路径。",
    steps: ["先点“发现版本”刷新受管和外部候选。", "JDK 问题优先点“检查当前 JDK”或对外部 JDK 做只读验证。", "确认 java/javac/jar 都可用后，再生成 JAVA_HOME 稳定计划。"],
    readonly: ["发现版本、外部 JDK 验证、java/javac/jar 检查是只读。", "外部 JDK 不会被卸载、移动或接管。"],
    writes: ["切换受管运行时会更新 current 指针，并可能配合环境页写入用户环境变量。", "安装受管版本会下载到 DevEnv 管理目录。"],
    safety: ["写入环境变量前会先生成计划和备份。", "IDE 捆绑运行时默认只展示和验证，不作为卸载目标。"],
  },
  environment: {
    title: "环境变量",
    intro: "环境变量页面专门检查用户级 JAVA_HOME、DEVENV_HOME、PATH 顺序、java/javac/pip 命中和 Maven/Gradle 使用的 Java。适合处理终端里版本和软件界面看到的不一致。",
    steps: ["先点“检查可靠性”。", "确认冲突来源后再生成修复计划。", "应用计划前核对 diff、备份名和是否需要重启终端。"],
    readonly: ["可靠性检查、修复计划预览、备份列表查看、报告导出是只读。", "页面会展示 raw 值、展开后的路径和命令验证结果。"],
    writes: ["应用计划会写当前用户环境变量。", "恢复环境会用最近备份替换用户级配置。"],
    safety: ["应用、恢复和 PATH 清理都需要后端 token。", "如果预览后环境变量发生变化，后端会拒绝写入并要求重新预览。"],
  },
  project: {
    title: "项目",
    intro: "项目页用于识别项目类型、读取 IDE 配置、验证 Java 消费者、生成 VS Code/IDEA 配置预览，以及备份后修改项目端口。适合启动项目、接手项目或排查端口冲突前使用。",
    steps: ["先选择项目目录；默认不会自动填本机路径。", "点击“分析”识别项目类型和运行建议。", "需要写配置或改端口时，先生成预览并核对文件内容。"],
    readonly: ["项目分析、IDEA 配置读取、Nacos/Nexus Java 验证和端口配置扫描是只读。", "页面只读取常见安全配置文件，不深扫源码内容。"],
    writes: ["应用项目配置会写 VS Code/IDEA 等项目配置文件。", "修改端口会备份原文件并只替换识别到的端口项。"],
    safety: ["写项目文件和端口修改都需要 token。", "后端会限制写入路径，避免越界修改项目外文件。"],
  },
  toolchains: {
    title: "工具链",
    intro: "工具链页面向 Git、SSH、Node 包管理器、Python/pip 和常用 CLI 配置。适合首次配置开发机、修复 pip/npm 源或确认命令是否可用。",
    steps: ["先点“全面检查”。", "Git 身份、SSH key、pip/npm 源按页面提示逐项处理。", "执行前阅读会写入哪些用户配置文件。"],
    readonly: ["检查命令版本、读取配置状态、复制建议命令是只读。", "命令输出会做基础脱敏。"],
    writes: ["保存 Git 身份、生成 SSH key、切换 pip/npm/chsrc 源会写用户配置。", "受管 pip 修复会先生成计划再执行。"],
    safety: ["写配置前会提示影响范围，必要时生成备份。", "失败后优先复制错误和命令输出，不要手动删除配置目录。"],
  },
  platforms: {
    title: "平台与镜像",
    intro: "平台页用于检查 Go、Rust、.NET、chsrc 和镜像源配置。它更像开发平台体检，不替代各生态成熟包管理器。",
    steps: ["先点“全面检查”。", "根据生态选择 Go/Rust/.NET 或 chsrc 操作。", "切换镜像前确认团队或项目是否有固定要求。"],
    readonly: ["版本检查、镜像测速和配置读取是只读。", "不会自动安装或卸载生态运行时。"],
    writes: ["镜像切换会写对应工具的用户配置文件。", "chsrc 操作会调用受控白名单命令。"],
    safety: ["写入前会备份或提示可恢复路径。", "如果公司网络有代理/内网源，先复制现有配置再改。"],
  },
  learning: {
    title: "学习中心",
    intro: "学习中心只运行固定白名单里的只读检查命令，帮助你理解 where、version、doctor 等命令输出。适合学习排查思路，而不是执行修复。",
    steps: ["选择预置命令或输入只读命令。", "运行后看 stdout/stderr 和安全评估。", "把输出带到其它页面决定下一步。"],
    readonly: ["允许的命令只用于查看版本、路径和诊断信息。", "被拒绝的命令会说明原因。"],
    writes: ["学习中心不安装工具、不改配置、不清理文件、不结束进程。"],
    safety: ["PowerShell/cmd、破坏性 Git、磁盘/注册表/权限类命令会被拦截。", "不要粘贴看不懂的网页命令。"],
  },
  maintenance: {
    title: "空间分析",
    intro: "空间分析用于做 C 盘只读体检、扫描低风险缓存、查看桌面/下载目录大文件、重复文件、常见应用占用，并在确认后生成清理或归档计划。适合先找证据，再少量、安全地释放空间。",
    steps: ["先点“开始体检”看总体风险。", "桌面急救和下载目录先用“只读分析”，分页查看分类占用和 Top 文件。", "只把确认不需要的项目加入计划，再预览清理或归档。"],
    readonly: ["体检、扫描、文件定位、复制路径、重复候选分析和应用占用统计都是只读。", "桌面/下载明细只展示文件名、路径、目录、大小、修改时间、类型和定位状态。"],
    writes: ["清理会重新校验选中项，普通文件进入回收站或调用官方缓存命令。", "归档/搬家会先生成计划，展示源、目标、估算大小、风险和回滚信息。"],
    safety: ["不会自动删除未选择文件，不会读取数据库正文或浏览器凭据。", "执行清理、搬家、回滚和扩容计划都需要 token；失败后先重新扫描。"],
  },
  toolbox: {
    title: "工具箱",
    intro: "工具箱承载命令面板、更新、本地服务、Docker/WSL 和卸载等高级入口。它适合有明确目标时使用，不建议把这里当作一键系统管家。",
    steps: ["先展开对应高级区并阅读说明。", "服务和 Docker/WSL 操作前确认目标名称、端口和当前状态。", "更新前先检查版本，再下载并校验安装包。"],
    readonly: ["服务检查、日志读取、Docker/WSL 状态检查、更新检查是只读。", "打开系统位置或复制日志不会修改状态。"],
    writes: ["启动/停止服务、Docker/WSL 安装更新、下载更新、自卸载会改变系统状态或打开系统工具。", "自卸载只打开 Windows 卸载器并关闭程序，不主动删除项目、数据库或运行时目录。"],
    safety: ["系统级动作折叠在高级区，并要求 token 或明确确认。", "失败后不要连续重复点击，先复制日志或错误信息。"],
  },
};

function activeView() {
  return document.querySelector(".nav-item.active")?.getAttribute("data-view") || "overview";
}

function riskInfoForView(view: string, featureRisks: FeatureRiskInfo[]) {
  const featureId = VIEW_FEATURE_MAP[view] || "overview";
  return featureRisks.find((item) => item.featureId === featureId);
}

export function renderViewGuide(
  view = activeView(),
  featureRisks: FeatureRiskInfo[] = [],
  escapeHtml: (value: string) => string,
) {
  const guide = document.querySelector<HTMLElement>("#view-guide-text");
  if (!guide) return;
  const definition = VIEW_GUIDES[view] || VIEW_GUIDES.overview;
  const info = riskInfoForView(view, featureRisks);
  const riskHtml = info
    ? `<section><h4>风险与边界</h4><ul>
        <li>风险等级：${escapeHtml(info.riskLevel)}；确认级别：${escapeHtml(info.confirmationLevel === "none" ? "无需确认" : info.confirmationLevel === "triple" ? "三次确认" : "二次确认")}。</li>
        <li>${info.requiresBackup ? "执行前需要备份或生成可恢复记录。" : "主要是只读或低风险动作，通常不需要备份。"}</li>
        ${info.whatItDoes.map((item) => `<li>能做：${escapeHtml(item)}</li>`).join("")}
        ${info.whatItDoesNotDo.map((item) => `<li>不会做：${escapeHtml(item)}</li>`).join("")}
      </ul></section>`
    : "";
  guide.innerHTML = `
    <section><h3>${escapeHtml(definition.title)}</h3><p>${escapeHtml(definition.intro)}</p></section>
    <section><h4>建议流程</h4><ol>${definition.steps.map((item) => `<li>${escapeHtml(item)}</li>`).join("")}</ol></section>
    <section><h4>只读能力</h4><ul>${definition.readonly.map((item) => `<li>${escapeHtml(item)}</li>`).join("")}</ul></section>
    <section><h4>会修改什么</h4><ul>${definition.writes.map((item) => `<li>${escapeHtml(item)}</li>`).join("")}</ul></section>
    <section><h4>安全与失败处理</h4><ul>${definition.safety.map((item) => `<li>${escapeHtml(item)}</li>`).join("")}</ul></section>
    ${riskHtml}
  `;
}

export function clearFeatureHelp() {
  const slot = document.querySelector<HTMLElement>("#feature-help-slot");
  if (slot) slot.innerHTML = "";
}
