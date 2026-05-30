import type { CodexState, CodexTargetsResponse, ProbeResult, CodexHistoryEntry, ProviderConfigsResponse, SetupSnippetsResponse } from "@/types/codex"

const API_BASE = "/admin/api"

async function fetchJson<T>(url: string, options?: RequestInit): Promise<T> {
  const response = await fetch(url, {
    ...options,
    headers: {
      "Content-Type": "application/json",
      ...options?.headers,
    },
  })
  if (!response.ok) {
    throw new Error(`HTTP ${response.status}: ${response.statusText}`)
  }
  return response.json() as Promise<T>
}

// Backend wraps many responses in { ok: true, data: { ... } }
interface WrappedResponse<T> {
  ok: boolean
  data: T
  error?: string | null
}

interface ProviderHealthRow {
  provider_id: string
  provider_name?: string  // Optional - derive from provider_id if not present
  requests: number
  errors: number
  error_rate: number
  avg_latency_ms?: number  // Optional - may not be present
  last_request_ts?: number | null  // Optional - may be last_seen
  last_seen?: number | null  // Backend may return last_seen instead
}

function normalizeHealthRow(row: any): ProviderHealthRow {
  return {
    provider_id: row.provider_id,
    provider_name: row.provider_name || row.provider_id,
    requests: row.requests || 0,
    errors: row.errors || 0,
    error_rate: row.error_rate || 0,
    avg_latency_ms: row.avg_latency_ms ?? row.latency_ms ?? 0,
    last_request_ts: row.last_request_ts ?? row.last_seen ?? null,
  }
}

export interface ApiResponse<T> {
  ok: boolean
  data?: T
  error?: string | null
}

export const api = {
  // Data Directory API
  dataDir: {
    // GET /admin/api/data-dir/info
    info: () =>
      fetchJson<{
        current: string
        defaultDir: string
        editable: boolean
        source: string
      }>(`${API_BASE}/data-dir/info`),

    // POST /admin/api/data-dir/preview
    preview: (targetDir: string) =>
      fetchJson<{
        ok: boolean
        currentDir: string
        targetDir: string
        estimatedBytes: number
        exists: boolean
      }>(`${API_BASE}/data-dir/preview`, {
        method: "POST",
        body: JSON.stringify({ targetDir }),
      }),
  },

  // Bootstrap API
  bootstrap: {
    // GET /admin/api/bootstrap-status
    status: () =>
      fetchJson<{
        needsBootstrap: boolean
        userCount: number
      }>(`${API_BASE}/bootstrap-status`),

    // POST /admin/api/bootstrap
    setup: (username: string, password: string, displayName?: string) =>
      fetchJson<{
        ok: boolean
        userId: number
        isAdmin: boolean
        username: string
      }>(`${API_BASE}/bootstrap`, {
        method: "POST",
        body: JSON.stringify({ username, password, display_name: displayName }),
      }),
  },

  // Auth API
  auth: {
    // POST /admin/api/auth/login
    login: (username: string, password: string) =>
      fetchJson<{
        ok: boolean
        data?: {
          token: string
          user: {
            id: number
            username: string
            role: string
            is_admin: boolean
            created_at: number
          }
          expires_at: number
        }
        error?: string | null
      }>(`${API_BASE}/auth/login`, {
        method: "POST",
        body: JSON.stringify({ username, password }),
      }),

    // POST /admin/api/auth/register
    register: (username: string, password: string) =>
      fetchJson<{
        ok: boolean
        data?: {
          token: string
          user: {
            id: number
            username: string
            role: string
            is_admin: boolean
            created_at: number
          }
          expires_at: number
        }
        error?: string | null
      }>(`${API_BASE}/auth/register`, {
        method: "POST",
        body: JSON.stringify({ username, password }),
      }),

    // POST /admin/api/auth/logout
    logout: () =>
      fetchJson<{ ok: boolean }>(`${API_BASE}/auth/logout`, { method: "POST" }),

    // GET /admin/api/auth/me (current user info)
    me: () =>
      fetchJson<{
        ok: boolean
        data?: {
          id: number
          username: string
          role: string
          is_admin: boolean
          created_at: number
        }
        error?: string | null
      }>(`${API_BASE}/auth/me`),
  },

  // Users API
  users: {
    // GET /admin/api/users
    list: () =>
      fetchJson<{
        users: Array<{
          id: number
          username: string
          displayName: string | null
          email: string | null
          avatarUrl: string | null
          isAdmin: boolean
          status: string
          createdAt: string
          updatedAt: string
          requestCount: number
          totalTokens: number
          lastActivity: string | null
        }>
      }>(`${API_BASE}/users`),

    // POST /admin/api/users
    create: (username: string, password: string, isAdmin: boolean = false) =>
      fetchJson<{
        user: {
          id: number
          username: string
          isAdmin: boolean
          status: string
        }
      }>(`${API_BASE}/users`, {
        method: "POST",
        body: JSON.stringify({ username, password, is_admin: isAdmin }),
      }),

    // PATCH /admin/api/users/:id
    update: (id: number, updates: { role?: string; is_admin?: boolean; status?: string; password?: string }) =>
      fetchJson<{
        user: {
          id: number
          username: string
          isAdmin: boolean
          status: string
        }
      }>(`${API_BASE}/users/${id}`, {
        method: "PATCH",
        body: JSON.stringify(updates),
      }),

    // DELETE /admin/api/users/:id
    delete: (id: number) =>
      fetchJson<{ ok: boolean }>(`${API_BASE}/users/${id}`, {
        method: "DELETE",
      }),
  },

  // Stats API
  stats: {
    // GET /admin/api/stats?range=24h
    // Backend returns: { total_providers: number, uptime_seconds: number }
    get: (range: string) =>
      fetchJson<{
        total_providers: number
        uptime_seconds: number
      }>(`${API_BASE}/stats?range=${range}`),

    // GET /admin/api/request-stats?range=24h
    // Backend returns: { ok: true, data: { since, rows: [...] } }
    // Rows have: provider_id, upstream_model, requests, errors, prompt_tokens, completion_tokens, total_tokens
    requestStats: (range: string) =>
      fetchJson<{
        ok: boolean
        data: {
          since: number
          rows: Array<{
            provider_id: string
            upstream_model: string
            requests: number
            errors: number
            prompt_tokens: number
            completion_tokens: number
            total_tokens: number
          }>
        }
      }>(`${API_BASE}/request-stats?range=${range}`).then(r => r.data ?? { since: 0, rows: [] }),

    // GET /admin/api/stats/timeseries?range=24h&bucket=hour
    // Backend returns: { range: string, series: [...] }
    tokenTimeseries: (range: string, bucket: string) =>
      fetchJson<{
        range: string
        series: Array<{
          provider_id: string
          model: string
          points: Array<{
            ts: number
            prompt_tokens: number
            completion_tokens: number
            cached_tokens: number
          }>
        }>
      }>(`${API_BASE}/stats/timeseries?range=${range}&bucket=${bucket}`),

    // GET /admin/api/stats/errors?range=24h
    // Backend returns: { rows: [{ error_code, count }] } or { rows: [{ code, count }] }
    errorStats: (range: string) =>
      fetchJson<{
        rows?: Array<{
          code?: string
          error_code?: string
          count: number
          percentage?: number
        }>
      }>(`${API_BASE}/stats/errors?range=${range}`).then(r => ({
        error_codes: (r.rows || []).map((row: any) => ({
          code: row.error_code || row.code || "unknown",
          count: row.count
        }))
      })),

    // GET /admin/api/stats/latency?range=24h
    // Backend returns: { range: string, stats: { avgMs, count, maxMs, minMs } }
    latencyStats: (range: string) =>
      fetchJson<{
        range: string
        stats: {
          avgMs: number
          count: number
          maxMs: number
          minMs: number
        }
      }>(`${API_BASE}/stats/latency?range=${range}`),

    // GET /admin/api/provider-health
    // Backend returns: { rows: [...] }
    providerHealth: () =>
      fetchJson<{
        rows: Array<ProviderHealthRow>
      }>(`${API_BASE}/provider-health`).then(r => ({
        rows: (r.rows || []).map(normalizeHealthRow)
      })),
  },

  // Logs API
  logs: {
    // GET /admin/api/logs?limit=10&offset=0
    // Backend returns: { ok: true, data: { logs: [...] } }
    list: (params: {
      provider?: string
      model?: string
      statusMin?: number
      statusMax?: number
      limit?: number
      offset?: number
    }) => {
      const searchParams = new URLSearchParams()
      if (params.provider) searchParams.set("provider", params.provider)
      if (params.model) searchParams.set("model", params.model)
      if (params.statusMin !== undefined) searchParams.set("status_min", String(params.statusMin))
      if (params.statusMax !== undefined) searchParams.set("status_max", String(params.statusMax))
      if (params.limit) searchParams.set("limit", String(params.limit))
      if (params.offset) searchParams.set("offset", String(params.offset))
      return fetchJson<WrappedResponse<{
        logs: Array<{
          id: number
          ts: number
          provider_id: string
          client_model: string
          upstream_model: string
          endpoint: string
          status_code: number
          duration_ms: number
          prompt_tokens: number | null
          completion_tokens: number | null
          total_tokens: number | null
          stream: boolean
          error_code: string | null
          error_snippet: string | null
        }>
      }>>(`${API_BASE}/logs?${searchParams.toString()}`).then(r => r.data ?? { logs: [] })
    },

    // GET /admin/api/logs/:id
    detail: (id: number) =>
      fetchJson<WrappedResponse<{
        log: {
          id: number
          ts: number
          provider_id: string
          client_model: string
          upstream_model: string
          endpoint: string
          status_code: number
          duration_ms: number
          prompt_tokens: number | null
          completion_tokens: number | null
          total_tokens: number | null
          stream: boolean
          error_code: string | null
          error_snippet: string | null
          request_body: string | null
          response_body: string | null
        }
      }>>(`${API_BASE}/logs/${id}`).then(r => r.data ?? { log: null }),

    // DELETE /admin/api/logs?before=timestamp
    deleteBefore: (before: number) =>
      fetchJson<WrappedResponse<{ removed: number }>>(`${API_BASE}/logs?before=${before}`, { method: "DELETE" })
        .then(r => r.data ?? { removed: 0 }),
  },

  // Providers API
  providers: {
    // GET /admin/api/providers
    // Backend returns: { zhipu: { id, name, ... }, ... }
    list: () =>
      fetchJson<Record<string, {
        id: string
        shortcut?: string
        name: string
        display_name: string
        default?: boolean
        enabled: boolean
        api_key_present?: boolean
        api_key_env?: string[]
        base_url?: string
        default_model?: string
      }>>(`${API_BASE}/providers`),

    // GET /admin/api/provider-configs
    // Backend returns: { ok: true, data: { providers: [...] } }
    config: () =>
      fetchJson<WrappedResponse<{
        providers: Array<{
          id: string
          shortcut: string
          display_name: string
          default: boolean
          enabled: boolean
          api_key_present: boolean
          api_key_env: string[]
          base_url: string
          default_model: string
        }>
      }>>(`${API_BASE}/provider-configs`).then(r => r.data ?? { providers: [] }),

    // GET /admin/api/providers/:id/models
    models: (id: string) =>
      fetchJson<WrappedResponse<{
        models: Array<{
          id: number
          upstream_id: string
          display_name: string | null
          context_window: number | null
          supports_images: boolean
          supports_reasoning: boolean
          supports_web_search: boolean
          is_builtin: boolean
          deprecated_after: string | null
        }>
      }>>(`${API_BASE}/providers/${id}/models`).then(r => r.data ?? { models: [] }),

    // Generic Providers CRUD
    create: (data: {
      id: string
      display_name: string
      endpoint_base: string
      auth_type: "bearer" | "api-key"
      api_key?: string
    }) =>
      fetchJson<WrappedResponse<{ created: boolean }>>(`${API_BASE}/generic-providers`, {
        method: "POST",
        body: JSON.stringify(data),
      }).then(r => r.data ?? { created: false }),

    update: (id: string, data: {
      display_name?: string
      endpoint_base?: string
      auth_type?: "bearer" | "api-key"
      api_key?: string
    }) =>
      fetchJson<WrappedResponse<{ updated: boolean }>>(`${API_BASE}/generic-providers`, {
        method: "PUT",
        body: JSON.stringify({ id, ...data }),
      }).then(r => r.data ?? { updated: false }),

    delete: (id: string) =>
      fetchJson<WrappedResponse<{ deleted: boolean }>>(`${API_BASE}/generic-providers?id=${id}`, {
        method: "DELETE",
      }).then(r => r.data ?? { deleted: false }),

    // GET /admin/api/provider-presets
    // Backend returns: { presets: [...] }
    presets: () =>
      fetchJson<{
        presets: Array<{
          id: string
          name: string
          shortcut: string
          defaultBaseUrl: string
          defaultModel: string
        }>
      }>(`${API_BASE}/provider-presets`),
  },

  // Models API (direct CRUD)
  models: {
    // GET /admin/api/providers/:id/models
    list: (providerId: string) =>
      fetchJson<{
        models: Array<{
          id: number
          upstream_id: string
          display_name: string | null
          context_window: number | null
          supports_images: boolean
          supports_reasoning: boolean
          supports_web_search: boolean
          is_builtin: boolean
          deprecated_after: string | null
        }>
      }>(`${API_BASE}/providers/${providerId}/models`).then(r => ({ models: r.models ?? [] })),

    create: (providerId: string, data: { upstream_id: string; display_name?: string }) =>
      fetchJson<WrappedResponse<{ created: boolean }>>(`${API_BASE}/providers/${providerId}/models`, {
        method: "POST",
        body: JSON.stringify(data),
      }).then(r => r.data ?? { created: false }),

    update: (id: number, data: { display_name?: string; deprecated_after?: string | null }) =>
      fetchJson<WrappedResponse<{ updated: boolean }>>(`${API_BASE}/models/${id}`, {
        method: "PATCH",
        body: JSON.stringify(data),
      }).then(r => r.data ?? { updated: false }),

    delete: (id: number) =>
      fetchJson<WrappedResponse<{ deleted: boolean }>>(`${API_BASE}/models/${id}`, { method: "DELETE" })
        .then(r => r.data ?? { deleted: false }),
  },

  // Account API
  account: {
    // GET /admin/api/me/api-keys
    apiKeys: () =>
      fetchJson<WrappedResponse<{
        api_keys: Array<{
          id: number
          name: string
          key_prefix: string
          created_at: number
          last_used_at: number | null
          revoked_at: number | null
        }>
      }>>(`${API_BASE}/me/api-keys`).then(r => r.data ?? { api_keys: [] }),

    createApiKey: (name: string) =>
      fetchJson<WrappedResponse<{ token: string; key_id: number }>>(`${API_BASE}/me/api-keys`, {
        method: "POST",
        body: JSON.stringify({ name }),
      }).then(r => r.data ?? { token: "", key_id: 0 }),

    revokeApiKey: (id: number) =>
      fetchJson<WrappedResponse<{ revoked: boolean }>>(`${API_BASE}/me/api-keys/${id}`, { method: "DELETE" })
        .then(r => r.data ?? { revoked: false }),

    // BYOK - Upstream Keys API
    // GET /admin/api/me/upstream-keys
    upstreamKeys: () =>
      fetchJson<WrappedResponse<{
        upstream_keys: Array<{
          provider_id: string
          has_key: boolean
          updated_at: number | null
        }>
      }>>(`${API_BASE}/me/upstream-keys`).then(r => r.data ?? { upstream_keys: [] }),

    // PUT /admin/api/me/upstream-keys/:providerId
    setUpstreamKey: (providerId: string, apiKey: string) =>
      fetchJson<WrappedResponse<{ ok: boolean; provider_id: string }>>(
        `${API_BASE}/me/upstream-keys/${encodeURIComponent(providerId)}`,
        {
          method: "PUT",
          body: JSON.stringify({ apiKey }),
        }
      ).then(r => r.data ?? { ok: false, provider_id: providerId }),

    // DELETE /admin/api/me/upstream-keys/:providerId
    deleteUpstreamKey: (providerId: string) =>
      fetchJson<WrappedResponse<{ deleted: boolean }>>(
        `${API_BASE}/me/upstream-keys/${encodeURIComponent(providerId)}`,
        { method: "DELETE" }
      ).then(r => r.data ?? { deleted: false }),

    // OAuth Clients API
    // GET /admin/api/oauth-clients
    oauthClients: () =>
      fetchJson<WrappedResponse<{
        clients: Array<{
          provider: "github" | "gitee"
          client_id: string
          callback_url: string
          enabled: boolean
          updated_at: number
          has_secret: boolean
        }>
      }>>(`${API_BASE}/oauth-clients`).then(r => r.data ?? { clients: [] }),

    // PUT /admin/api/oauth-clients/:provider
    saveOAuthClient: (
      provider: "github" | "gitee",
      data: { clientId: string; clientSecret?: string | null; callbackUrl: string; enabled: boolean }
    ) =>
      fetchJson<WrappedResponse<{ ok: boolean }>>(
        `${API_BASE}/oauth-clients/${provider}`,
        {
          method: "PUT",
          body: JSON.stringify({
            client_id: data.clientId,
            client_secret: data.clientSecret,
            callback_url: data.callbackUrl,
            enabled: data.enabled,
          }),
        }
      ).then(r => r.data ?? { ok: false }),

    // DELETE /admin/api/oauth-clients/:provider
    deleteOAuthClient: (provider: "github" | "gitee") =>
      fetchJson<WrappedResponse<{ deleted: boolean }>>(
        `${API_BASE}/oauth-clients/${provider}`,
        { method: "DELETE" }
      ).then(r => r.data ?? { deleted: false }),
  },

  codex: {
    state: () => fetchJson<ApiResponse<CodexState>>(`${API_BASE}/codex-state`),

    targets: () => fetchJson<ApiResponse<CodexTargetsResponse>>(`${API_BASE}/codex-targets`),

    apply: (providerId: string, modelId: string) =>
      fetchJson<ApiResponse<{ backup_ts: number }>>(`${API_BASE}/codex-apply`, {
        method: "POST",
        body: JSON.stringify({ provider_id: providerId, model_id: modelId }),
      }),

    restore: (ts: number) =>
      fetchJson<ApiResponse<{ ts: number }>>(`${API_BASE}/codex-restore`, {
        method: "POST",
        body: JSON.stringify({ ts }),
      }),

    history: () => fetchJson<ApiResponse<CodexHistoryEntry[]>>(`${API_BASE}/codex-history`),

    probe: (providerId: string, modelId: string) =>
      fetchJson<ApiResponse<ProbeResult>>(`${API_BASE}/probe`, {
        method: "POST",
        body: JSON.stringify({ provider_id: providerId, model_id: modelId }),
      }),

    thinking: () => fetchJson<ApiResponse<{ disabled: boolean; force_high_effort: boolean; cli_override: boolean }>>(`${API_BASE}/thinking`),

    setThinking: (disabled: boolean, forceHighEffort: boolean) =>
      fetchJson<ApiResponse<{ updated: boolean }>>(`${API_BASE}/thinking`, {
        method: "PUT",
        body: JSON.stringify({ disabled, force_high_effort: forceHighEffort }),
      }),

    activeOverride: () => fetchJson<ApiResponse<{ providerId?: string; modelId?: string }>>(`${API_BASE}/active-override`),

    setOverride: (providerId: string, modelId: string) =>
      fetchJson<ApiResponse<{ override: { providerId: string; modelId: string } }>>(`${API_BASE}/active-override`, {
        method: "PUT",
        body: JSON.stringify({ provider_id: providerId, model_id: modelId }),
      }),

    clearOverride: () => fetchJson<ApiResponse<{ override: null }>>(`${API_BASE}/active-override`, { method: "DELETE" }),

    currentBundle: () => fetchJson<ApiResponse<CodexBundleResponse>>(`${API_BASE}/codex-current-bundle`),

    import: (params: { authJson: string; configToml: string; providerId?: string; modelId?: string; note?: string }) =>
      fetchJson<ApiResponse<ImportResponse>>(`${API_BASE}/codex-import`, {
        method: "POST",
        body: JSON.stringify({
          auth_json: params.authJson,
          config_toml: params.configToml,
          provider_id: params.providerId,
          model_id: params.modelId,
          note: params.note,
        }),
      }),

    providerConfigs: () => fetchJson<ApiResponse<ProviderConfigsResponse>>(`${API_BASE}/provider-configs`),

    setupSnippets: () => fetchJson<ApiResponse<SetupSnippetsResponse>>(`${API_BASE}/setup-snippets`),

    deleteBackup: (ts: number) =>
      fetchJson<ApiResponse<{ deleted: boolean }>>(`${API_BASE}/codex-backups/${ts}`, { method: "DELETE" }),

    historyBundle: (id: number) =>
      fetchJson<ApiResponse<CodexBundleResponse>>(`${API_BASE}/codex-history/${id}/bundle`),

    getCodexDir: () =>
      fetchJson<ApiResponse<{ source: string; effective: string }>>(`${API_BASE}/codex-dir`),

    setCodexDir: (dir: string) =>
      fetchJson<ApiResponse<{ source: string; effective: string }>>(`${API_BASE}/codex-dir`, {
        method: "PUT",
        body: JSON.stringify({ dir }),
      }),

    clearCodexDir: () =>
      fetchJson<ApiResponse<{ source: string; effective: string }>>(`${API_BASE}/codex-dir`, {
        method: "DELETE",
      }),
  },
}

interface CodexBundleResponse {
  history: { id: number; ts: number; kind: string; provider_id?: string; model_id?: string; note?: string }
  files: { auth_json: string; config_toml: string }
  scripts: { posix: string; powershell: string }
}

interface ImportResponse {
  ok: boolean
  historyId: number
  restartRequired: boolean
  bundleUrl?: string | null
}
