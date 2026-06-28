export type AppSnapshot = {
  defaultRoot: string;
  configDir: string;
  os: string;
  arch: string;
  username: string;
};

export type EnvSnapshot = {
  pathEntries: string[];
  javaHome?: string;
  devenvHome?: string;
  pathWarnings: string[];
};

export type ConfigView = {
  settings: {
    rootDir: string;
    autoCheckUpdate: boolean;
    downloadTimeoutSeconds: number;
    theme: string;
    safetyDisclaimerAccepted: boolean;
    safetyDisclaimerVersion: number;
    safetyDisclaimerAcceptedAt?: string | null;
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

export type ManagedRuntime = {
  version: string;
  path: string;
  detail?: string;
  installed_at?: string;
  installedAt?: string;
};

export type OperationResult = {
  success: boolean;
  message: string;
};

export type ToolProbe = { path?: string; version: string; source: string };
export type EnvRepairAction = {
  id: string;
  title: string;
  description: string;
  variable: string;
  oldValue?: string;
  newValue?: string;
  risk: string;
  reversible: boolean;
};
export type EnvRepairPlan = {
  planId: string;
  createdAt: string;
  target: string;
  actions: EnvRepairAction[];
  expectedAfter: { javaHome?: string; devenvHome?: string; path?: string };
  warnings: string[];
  riskLevel: string;
  requiresTerminalRestart: boolean;
  backupName: string;
  disclaimer: string;
  diff: string[];
};
export type EnvRepairResult = {
  planId: string;
  success: boolean;
  message: string;
  backupName: string;
};
export type EnvBackupRecord = {
  backupName: string;
  createdAt: string;
  reason: string;
  variables: string[];
  javaHomePreview?: string;
  devenvHomePreview?: string;
  pathEntryCount: number;
  sourcePlanId?: string;
};
export type EnvReliabilitySnapshot = {
  generatedAt: string;
  userEnv: {
    javaHomeRaw?: string;
    javaHomeExpanded?: string;
    devenvHomeRaw?: string;
    devenvHomeExpanded?: string;
    pathRaw: string;
    pathEntries: Array<{ raw: string; expanded: string; exists: boolean; isDuplicate: boolean; isStaleDevenvEntry: boolean; containsJava: boolean; containsJavac: boolean; containsPython: boolean; containsPip: boolean; containsNode: boolean; containsNpm: boolean; risk: string }>;
  };
  processEnv: { javaHomeRaw?: string; javaHomeExpanded?: string; pathRaw: string };
  effectiveTools: { java: ToolProbe; javac: ToolProbe; python: ToolProbe; pip: ToolProbe; node: ToolProbe; npm: ToolProbe; maven: ToolProbe; gradle: ToolProbe; go: ToolProbe };
  pathAnalysis: { totalEntries: number; duplicateCount: number; missingCount: number; staleDevenvCount: number; javaEntryCount: number; pythonEntryCount: number; storeAliasDetected: boolean; pathTooLong: boolean; explanation: string[] };
  java: { javaHomeRaw?: string; javaHomeExpanded?: string; javaHomeValid: boolean; pathJava?: string; pathJavac?: string; commandJavaVersion: string; commandJavacVersion: string; consistency: string; conflicts: string[]; candidates: Array<{ path: string; version: string; source: string }> };
  python: { currentPython?: ToolProbe; currentPip?: ToolProbe; pyLauncherOutput: string; discoveredPythons: Array<{ path: string; version: string; source: string }>; discoveredPips: Array<{ path: string; version: string; source: string }>; storeAliasRisk: boolean; pipMatchesPython: boolean; userPathEffective: boolean; conflicts: string[]; suggestions: string[] };
  mavenGradle: { mavenPath?: string; mavenVersion: string; mavenJava: string; gradlePath?: string; gradleVersion: string; gradleJava: string; conflicts: string[]; suggestions: string[] };
  node: { nodePath?: string; nodeVersion: string; npmPath?: string; npmVersion: string; npmPrefix: string; npmRegistry: string; pnpmStore: string; conflicts: string[]; suggestions: string[] };
  issues: Array<{ id: string; title: string; severity: string; detail: string }>;
  suggestions: Array<{ id: string; title: string; detail: string; action?: string }>;
};

export type FeatureRiskInfo = {
  featureId: string;
  title: string;
  riskLevel: string;
  whatItDoes: string[];
  whatItDoesNotDo: string[];
  possibleImpact: string[];
  reversible: boolean;
  requiresBackup: boolean;
  requiresAdmin: boolean;
  confirmationLevel: string;
  safeAlternatives: string[];
};

export type ValidationCheck = {
  id: string;
  title: string;
  success: boolean;
  required: boolean;
  detail: string;
  stage: string;
};

export type PythonIntegrityReport = {
  pythonPath: string;
  pythonHome: string;
  managed: boolean;
  fullyUsable: boolean;
  status: string;
  checks: ValidationCheck[];
  risks: string[];
  suggestions: string[];
};

export type RuntimeStrongVerificationReport = {
  generatedAt: string;
  items: Array<{
    kind: string;
    version: string;
    path: string;
    registered: boolean;
    current: boolean;
    environmentEffective: boolean;
    status: string;
    checks: ValidationCheck[];
    failureStage?: string;
    report: string[];
  }>;
  summary: string[];
};

export type IdeaProjectReport = {
  root: string;
  detected: boolean;
  readFiles: string[];
  projectSdk: string;
  languageLevel: string;
  moduleSdks: string[];
  moduleCount: number;
  compilerTarget: string;
  mavenImporterJdk: string;
  gradleJvm: string;
  outputDir: string;
  currentJavaHome: string;
  currentJavaVersion: string;
  jdkMatch: string;
  warnings: string[];
};

export type JavaConsumerReport = {
  consumer: string;
  root: string;
  startupExists: boolean;
  javaHomeRaw?: string;
  javaHomeExpanded?: string;
  javaExists: boolean;
  javacExists: boolean;
  pathJava?: string;
  indirectJavaHomeRisk: boolean;
  processUserEnvDiffers: boolean;
  usable: boolean;
  explanation: string[];
};

export type KillResult = OperationResult & {
  needsForce: boolean;
  blocked: boolean;
};

export type RuntimeInfo = {
  kind: string;
  version: string;
  executable: string;
  source: string;
};

export type JavaEnvironmentReport = {
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

export type PortRecord = {
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
  identity: string;
  confidence: number;
  evidenceCount: number;
  conflictCount: number;
  riskLevel: string;
  recommendation: string;
  evidence: string[];
  conflictEvidence: string[];
};

export type PortHistorySummary = {
  port: number;
  processName: string;
  observations: number;
  lastSeen: number;
};

export type PortSortKey = "localPort" | "state" | "identity" | "processName" | "pid" | "confidence" | "riskLevel";
export type SortDirection = "asc" | "desc";

export type ProjectHealth = {
  root: string;
  projectTypes: string[];
  signals: string[];
  suggestions: string[];
};

export type TaskProgress = {
  task: string;
  percent: number;
  message: string;
};

export type NetworkDiagnostics = {
  checks: Array<{
    name: string;
    url: string;
    success: boolean;
    status: string;
    elapsedMs: number;
  }>;
  proxy: Array<[string, string]>;
};

export type CacheEntry = {
  name: string;
  path: string;
  size: number;
  sha256?: string;
};

export type CommandRunResult = {
  success: boolean;
  returnCode: number;
  output: string;
  elapsedMs: number;
};

export type CommandSafetyAssessment = {
  allowed: boolean;
  risk: string;
  reason: string;
  requiresConfirmation: boolean;
  elevated: boolean;
  executable: string;
};

export type AgentTraceReport = {
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

export type EnvHealthCheck = {
  name: string;
  status: string;
  detail: string;
};

export type ConfigProfile = {
  id: string;
  name: string;
  createdAt: string;
  current: Record<string, string | null>;
  devenvHome?: string;
  javaHome?: string;
  path: string;
};

export type DoctorReport = {
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

export type PythonAnalysis = {
  currentPython?: PythonToolState;
  currentPip?: PythonToolState;
  launcherPath: string;
  launcherOutput: string;
  firstPythonOnPath: string;
  firstPipOnPath: string;
  pythonMPipAvailable: boolean;
  managedPythonAvailable: boolean;
  discoveredPythons: PythonEntry[];
  discoveredPips: PythonEntry[];
  userPathEntryCount: number;
  currentTerminalMatchesUserPath: boolean;
  storeAliasRisk: boolean;
  repairBlockers: string[];
  recoveryActions: string[];
  diagnosticReport: string;
  risks: string[];
  recommendations: string[];
  pipRepairCommand: string;
  aliasSettingsCommand: string;
};

export type PythonRepairPlan = {
  planId: string;
  createdAt: string;
  pythonPath: string;
  actions: string[];
  commands: string[];
  pathAdded: string[];
  warnings: string[];
  backupName: string;
};

export type PythonToolState = {
  path: string;
  version: string;
  status: string;
  detail: string;
};

export type PythonEntry = {
  path: string;
  source: string;
  version: string;
  current: boolean;
};

export type ProjectAnalysis = {
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
export type CurrentVersions = {
  jdk?: string;
  python?: string;
  node?: string;
  maven?: string;
  gradle?: string;
  go?: string;
};
export type ProjectConfigFileDraft = {
  relativePath: string;
  content: string;
  existed: boolean;
  enabled: boolean;
};
export type ProjectConfigPreview = {
  projectPath: string;
  detectedTypes: string[];
  files: ProjectConfigFileDraft[];
  current: CurrentVersions;
  warnings: string[];
};
export type ProjectPortConfig = {
  id: string;
  kind: string;
  file: string;
  currentPort: number;
  line: number;
  description: string;
};


export type ToolState = {
  name: string;
  installed: boolean;
  version: string;
  path: string;
  detail: string;
};

export type ToolchainReport = {
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

export type PlatformReport = {
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
  chsrc: ToolState;
  chsrcRecovery: {
    missing: boolean;
    explanation: string[];
    scoopCommand: string;
    wingetCommand: string;
    officialUrl: string;
    fallbackFeatures: string[];
  };
  generatedAt: string;
};

export type SystemPlatformReport = {
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

export type LocalServiceStatus = {
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

export type MySqlCandidate = {
  id: string;
  status: string;
  versionHint: string;
  serviceName: string;
  serviceState: string;
  mysqldPath: string;
  myIniPath: string;
  basedir: string;
  datadir: string;
  port: number;
  portOccupied: boolean;
  portProcess: string;
  dataHealth: string;
  confidence: string;
  conclusionLevel: string;
  staticFileCheck: string;
  connectionCheck: string;
  systemSchemaCheck: string;
  reasoning: string[];
  backupManifest?: MySqlBackupManifestStatus | null;
  evidence: string[];
  nextSteps: string[];
  systemSchemaMissing: boolean;
  businessDatabases: string[];
  lastError: string;
  suggestions: string[];
  registrationCommand: string;
  consoleCommand: string;
};

export type MySqlBackupManifestStatus = {
  valid: boolean;
  reason: string;
  createdAt: number;
  expiresAt: number;
  destination: string;
  files: number;
  bytes: number;
  ibdata: boolean;
  frm: boolean;
  businessSchema: boolean;
  systemSchema: boolean;
  manifestPath: string;
};

export type MySqlRepairReport = {
  generatedAt: string;
  candidates: MySqlCandidate[];
  warnings: string[];
  privacyNotice: string;
};

export type MySqlRepairPlan = {
  planId: string;
  createdAt: string;
  candidateId: string;
  action: string;
  title: string;
  steps: string[];
  commands: string[];
  warnings: string[];
  requiresAdmin: boolean;
  requiresBackup: boolean;
  riskLevel: string;
  planFingerprint: string;
};

export type ConfirmationTokenView = {
  token: string;
  actionId: string;
  planId: string;
  riskLevel: string;
  expiresAt: number;
};

export type MySqlExecutionGuard = {
  actionId: string;
  planId: string;
  riskLevel: string;
  planFingerprint: string;
  backupRequired: boolean;
  backupReceipt?: string;
};

export type JdkDistribution = {
  id: string;
  name: string;
  recommended: boolean;
  supportsInstall: boolean;
  description: string;
};

export type UpdateCheckResult = {
  currentVersion: string;
  latestVersion: string;
  updateAvailable: boolean;
  date: string;
  notes: string[];
  downloadUrl: string;
  sha256: string;
  checkedAt: string;
};

export type CleanupArchitecture = {
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

export type DoctorRepairResult = {
  beforeScore: number;
  afterScore: number;
  applied: string[];
  remaining: string[];
  report: DoctorReport;
};

export type ConfigProfileImportPreview = {
  source: string;
  exportedAt: string;
  profiles: Array<{
    name: string;
    current: Record<string, string | null>;
    missing: string[];
    willReplace: boolean;
  }>;
};

export type ProfileRequirement = {
  kind: string;
  version: string;
  installed: boolean;
  autoInstallSupported: boolean;
};

export type CleanupCandidate = {
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

export type CleanupCategoryScan = {
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

export type CleanupScanReport = {
  generatedAt: string;
  totalBytes: number;
  totalItems: number;
  categories: CleanupCategoryScan[];
  warnings: string[];
};

export type CleanupPlan = {
  planId: string;
  createdAt: string;
  selectedItems: Array<{
    itemId: string;
    path: string;
    size: number;
    categoryId: string;
    risk: string;
    action: string;
    reversible: boolean;
  }>;
  estimatedBytes: number;
  riskSummary: string[];
  requiresAdmin: boolean;
  warnings: string[];
};

export type CleanupResult = {
  planId: string;
  startedAt: string;
  finishedAt: string;
  success: boolean;
  cleanedBytes: number;
  cleanedItems: number;
  skippedItems: number;
  failedItems: number;
  failures: Array<{ path: string; reason: string }>;
  reportMarkdown: string;
};

export type MovePlan = {
  planId: string;
  createdAt: string;
  source: string;
  target: string;
  mode: string;
  estimatedBytes: number;
  itemCount: number;
  risk: string;
  requiresAdmin: boolean;
  reversible: boolean;
  warnings: string[];
};

export type MoveResult = {
  planId: string;
  success: boolean;
  movedBytes: number;
  movedItems: number;
  sourceBackup?: string;
  targetPath: string;
  junctionCreated: boolean;
  failures: string[];
  rollbackId?: string;
  reportMarkdown: string;
};

export type RollbackRecord = {
  rollbackId: string;
  createdAt: string;
  operationType: string;
  source: string;
  target: string;
  backupPath?: string;
  junctionPath?: string;
  reversible: boolean;
  notes: string[];
};

export type PartitionInfo = {
  diskIndex: string;
  partitionIndex: string;
  driveLetter?: string;
  size: number;
  fileSystem?: string;
  partitionType: string;
  isBoot: boolean;
  isSystem: boolean;
  isRecovery: boolean;
  isEmpty: boolean;
};

export type PartitionLayoutReport = {
  systemDisk: string;
  cPartition: PartitionInfo;
  adjacentRight?: PartitionInfo;
  unallocatedAfterC?: number;
  recoveryPartitionBlocks: boolean;
  dPartitionSameDisk: boolean;
  bitlockerSuspected: boolean;
  canExtendSafely: boolean;
  canDeleteEmptyAdjacentPartition: boolean;
  resultLevel: string;
  explanation: string;
  suggestedActions: string[];
};

export type ExpansionPlan = {
  planId: string;
  mode: string;
  canExecute: boolean;
  requiresAdmin: boolean;
  estimatedAddedBytes: number;
  commandsPreview: string[];
  risks: string[];
  backupRequired: boolean;
  explanation: string;
};

export type ExpansionResult = {
  planId: string;
  success: boolean;
  beforeFree: number;
  afterFree: number;
  beforeTotal: number;
  afterTotal: number;
  output: string;
  reportMarkdown: string;
};

export type DiskVolumeInfo = {
  drive: string;
  totalBytes: number;
  freeBytes: number;
  usedBytes: number;
  usedPercent: number;
  fileSystem?: string;
  risk: string;
};

export type MaintenanceOverview = {
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


export type LargeFileItem = {
  path: string;
  size: number;
  modifiedAt?: string;
  fileType: string;
  suggestion: string;
  risk: string;
};

export type ArchivePlanItem = {
  id: string;
  path: string;
  size: number;
  source: string;
  addedAt: string;
  suggestion: string;
};

export type DuplicateGroup = {
  size: number;
  hash: string;
  files: Array<{ path: string; modifiedAt?: string; keepSuggestion: string }>;
  reclaimableEstimate: number;
};

export type FolderUsageReport = {
  name: string;
  path: string;
  totalBytes: number;
  categories: Array<{ name: string; path: string; size: number; category: string; suggestion: string }>;
  suggestions: string[];
  warnings: string[];
};

export type InstalledSoftwareUsage = {
  name: string;
  publisher: string;
  installLocation: string;
  estimatedSize: number;
  uninstallCommandExists: boolean;
  suggestion: string;
};

export type AppUsageItem = {
  name: string;
  detected: boolean;
  path: string;
  size: number;
  categories: FolderUsageReport["categories"];
  safeActions: string[];
  warnings: string[];
};

export type AppUsageReport = {
  wechat?: AppUsageItem;
  qq?: AppUsageItem;
  browsers: AppUsageItem[];
  netDisks: AppUsageItem[];
  videoEditors: AppUsageItem[];
  gamePlatforms: AppUsageItem[];
  installedSoftware: InstalledSoftwareUsage[];
};

export type EnvironmentConfigPreview = {
  previewId: string;
  createdAt: string;
  changes: Array<{ name: string; current: string; proposed: string; impact: string }>;
  pathAdded: string[];
  pathRemoved: string[];
  warnings: string[];
  backupName: string;
};

export type EnvironmentBackupInfo = {
  fileName: string;
  createdAt: string;
  devenvHome: string;
  javaHome: string;
  pathEntries: number;
};

