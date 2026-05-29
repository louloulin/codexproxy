// Codex API Types

export interface ApiResponse<T> {
  ok: boolean
  data?: T
  error?: string | null
}

export interface CodexState {
  codexDir: string
  codexDirSource: "default" | "env" | "user"
  authJsonOwner: "mimo2codex" | "external" | "missing"
  authJsonExists: boolean
  authPath: string
  configTomlExists: boolean
  configTomlText: string | null
  backupPairs: BackupPair[]
}

export interface BackupPair {
  ts: number
  authBackup: string | null
  tomlBackup: string | null
  preserved: boolean
  model: string | null
  provider: string | null
}

export interface CodexTarget {
  providerId: string
  providerName: string
  modelId: string
  baseUrl: string
  hasKey: boolean
  displayName?: string
  source: string
  contextWindow?: number
  isCurrentOverride: boolean
}

export interface CodexTargetsResponse {
  targets: CodexTarget[]
}

export interface ProbeResult {
  ok: boolean
  latencyMs: number
  error?: {
    code: string
    message: string
  }
}

export interface ApplyCodexResponse {
  backupTs: number
  authBackup: string | null
  tomlBackup: string | null
  authJsonOwnerBefore: string
  preserved: boolean
}

export interface CodexHistoryEntry {
  id: number
  userId: number | null
  kind: "Initial" | "Apply" | "Restore"
  authJson: string
  configToml: string
  note: string | null
  createdAt: number
}

export interface ActiveOverride {
  providerId: string
  modelId: string
}

export interface CodexDirInfo {
  source: "default" | "env" | "user"
  effective: string
}

// Provider Configs API
export interface ProviderInfo {
  id: string
  shortcut: string
  display_name: string
  default: boolean
  enabled: boolean
  api_key_present: boolean
  api_key_env: string[]
  base_url: string
  default_model: string
}

export interface ProviderConfigsResponse {
  providers: ProviderInfo[]
}

// Setup Snippets API
export interface SetupSnippetTarget {
  provider_id: string
  provider_key: string
  provider_label: string
  model_id: string
  context_window?: number
  max_output_tokens?: number
}

export interface SetupSnippetBundle {
  target: SetupSnippetTarget
  auth_json: string
  config_toml: string
  config_toml_env_key: string
}

export interface ProviderSnippetInfo {
  id: string
  shortcut: string
  display_name: string
}

export interface SetupSnippetsResponse {
  bundle: SetupSnippetBundle
  default_provider_id: string
  providers: ProviderSnippetInfo[]
}
