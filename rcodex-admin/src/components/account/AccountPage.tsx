import { useTranslation } from "react-i18next"
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Input } from "@/components/ui/input"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { useState } from "react"
import { Copy, Trash2, Plus, Eye, EyeOff, Save } from "lucide-react"
import { api } from "@/lib/api"
import { PageTour, type TourStep } from "@/components/PageTour"

// Account PageTour steps
const ACCOUNT_TOUR_STEPS: TourStep[] = [
  {
    target: "[data-tour='account-api-keys']",
    title: "API Keys",
    description: "Create API keys for programmatic access to rcodex. Copy the key immediately after creation - it won't be shown again.",
    placement: "bottom",
  },
  {
    target: "[data-tour='account-byok']",
    title: "Bring Your Own Key",
    description: "Configure your own API keys for each provider instead of using shared keys. Useful for cost tracking.",
    placement: "bottom",
  },
  {
    target: "[data-tour='account-oauth']",
    title: "OAuth Configuration",
    description: "Configure GitHub or Gitee OAuth for team authentication. Enable OAuth to allow team members to login.",
    placement: "top",
  },
]

export function AccountPage() {
  const { t } = useTranslation()

  return (
    <PageTour pageKey="account" steps={ACCOUNT_TOUR_STEPS}>
    <div className="space-y-6">
      {/* Page Header */}
      <div>
        <h1 className="text-3xl font-bold tracking-tight">{t("account.title", "Account")}</h1>
        <p className="text-muted-foreground">
          {t("account.subtitle", "Manage your API keys and account settings")}
        </p>
      </div>

      {/* API Keys Section */}
      <div data-tour="account-api-keys">
      <ApiKeysSection />

      </div>

      {/* BYOK Section */}
      <div data-tour="account-byok">
      <ByokSection />

      </div>

      {/* OAuth Section (Admin only) */}
      <div data-tour="account-oauth">
      <OAuthSection />
      </div>
    </div>
    </PageTour>
  )
}

function ApiKeysSection() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const [newKeyName, setNewKeyName] = useState("")
  const [revealed, setRevealed] = useState<string | null>(null)

  const { data, isLoading } = useQuery({
    queryKey: ["api-keys"],
    queryFn: () => api.account.apiKeys(),
  })

  const createMutation = useMutation({
    mutationFn: (name: string) => api.account.createApiKey(name),
    onSuccess: (result) => {
      setRevealed(result.token)
      setNewKeyName("")
      queryClient.invalidateQueries({ queryKey: ["api-keys"] })
    },
    onError: (e) => {
      console.error("Failed to create API key:", e)
    },
  })

  const revokeMutation = useMutation({
    mutationFn: (id: number) => api.account.revokeApiKey(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["api-keys"] })
    },
    onError: (e) => {
      console.error("Failed to revoke API key:", e)
    },
  })

  async function createKey() {
    if (!newKeyName.trim()) return
    createMutation.mutate(newKeyName)
  }

  async function revokeKey(id: number) {
    if (!confirm(t("account.confirmRevoke", "Are you sure you want to revoke this key?"))) return
    revokeMutation.mutate(id)
  }

  const keys = data?.api_keys ?? []

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("account.apiKeys", "API Keys")}</CardTitle>
        <CardDescription>
          {t("account.apiKeysDesc", "Create and manage API keys for programmatic access")}
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Revealed Key Alert */}
        {revealed && (
          <div className="bg-emerald-50 border border-emerald-200 rounded-lg p-4">
            <p className="text-sm font-medium text-emerald-800 mb-2">
              {t("account.keyCreated", "API Key Created - Copy it now, you won't see it again!")}
            </p>
            <div className="flex items-center gap-2">
              <code className="flex-1 bg-white border rounded px-3 py-2 text-sm font-mono break-all">
                {revealed}
              </code>
              <Button
                size="sm"
                variant="outline"
                onClick={() => navigator.clipboard.writeText(revealed)}
              >
                <Copy className="h-4 w-4" />
              </Button>
            </div>
            <Button
              size="sm"
              variant="ghost"
              className="mt-2"
              onClick={() => setRevealed(null)}
            >
              {t("account.dismiss", "Dismiss")}
            </Button>
          </div>
        )}

        {/* Create Key Form */}
        <div className="flex gap-2">
          <Input
            placeholder={t("account.keyNamePlaceholder", "Key name (optional)")}
            value={newKeyName}
            onChange={(e) => setNewKeyName(e.target.value)}
            className="max-w-xs"
            onKeyDown={(e) => {
              if (e.key === "Enter") createKey()
            }}
          />
          <Button onClick={createKey} disabled={createMutation.isPending}>
            <Plus className="h-4 w-4 mr-1" />
            {createMutation.isPending ? t("account.creating", "Creating...") : t("account.createKey", "Create Key")}
          </Button>
        </div>

        {/* Keys Table */}
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{t("account.name", "Name")}</TableHead>
              <TableHead>{t("account.prefix", "Prefix")}</TableHead>
              <TableHead>{t("account.created", "Created")}</TableHead>
              <TableHead>{t("account.lastUsed", "Last Used")}</TableHead>
              <TableHead>{t("account.status", "Status")}</TableHead>
              <TableHead className="text-right">{t("account.actions", "Actions")}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {isLoading ? (
              <TableRow>
                <TableCell colSpan={6} className="text-center text-muted-foreground py-8">
                  {t("account.loading", "Loading...")}
                </TableCell>
              </TableRow>
            ) : keys.length === 0 ? (
              <TableRow>
                <TableCell colSpan={6} className="text-center text-muted-foreground py-8">
                  {t("account.noKeys", "No API keys yet. Create one above.")}
                </TableCell>
              </TableRow>
            ) : (
              keys.map((key) => (
                <TableRow key={key.id}>
                  <TableCell className="font-medium">{key.name || "Unnamed"}</TableCell>
                  <TableCell>
                    <code className="text-sm">{key.key_prefix}...</code>
                  </TableCell>
                  <TableCell className="text-muted-foreground">
                    {new Date(key.created_at * 1000).toLocaleDateString()}
                  </TableCell>
                  <TableCell className="text-muted-foreground">
                    {key.last_used_at
                      ? new Date(key.last_used_at * 1000).toLocaleDateString()
                      : t("account.never", "Never")}
                  </TableCell>
                  <TableCell>
                    <Badge variant={key.revoked_at ? "secondary" : "success"}>
                      {key.revoked_at ? t("account.revoked", "Revoked") : t("account.active", "Active")}
                    </Badge>
                  </TableCell>
                  <TableCell className="text-right">
                    {!key.revoked_at && (
                      <Button
                        size="sm"
                        variant="ghost"
                        className="text-destructive"
                        onClick={() => revokeKey(key.id)}
                        disabled={revokeMutation.isPending}
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    )}
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </CardContent>
    </Card>
  )
}

function ByokSection() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const [editing, setEditing] = useState<string | null>(null)
  const [value, setValue] = useState("")
  const [showKey, setShowKey] = useState(false)

  // Fetch providers list
  const { data: providersData } = useQuery({
    queryKey: ["providers-list"],
    queryFn: () => api.providers.list(),
  })

  // Fetch upstream keys
  const { data: upstreamKeysData, isLoading } = useQuery({
    queryKey: ["upstream-keys"],
    queryFn: () => api.account.upstreamKeys(),
  })

  // Normalize providers to array
  const providers: Array<{ id: string; display_name: string; api_key_env?: string[] }> = providersData
    ? Object.values(providersData as Record<string, { id: string; display_name?: string; api_key_env?: string[] }>).map(p => ({
        id: p.id,
        display_name: (p as any).display_name || p.id,
        api_key_env: (p as any).api_key_env,
      }))
    : []

  const upstreamKeys = upstreamKeysData?.upstream_keys ?? []

  const setKeyMutation = useMutation({
    mutationFn: async ({ providerId, apiKey }: { providerId: string; apiKey: string }) => {
      return api.account.setUpstreamKey(providerId, apiKey)
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["upstream-keys"] })
      setEditing(null)
      setValue("")
    },
    onError: (e) => {
      console.error("Failed to set upstream key:", e)
    },
  })

  const deleteKeyMutation = useMutation({
    mutationFn: async (providerId: string) => {
      return api.account.deleteUpstreamKey(providerId)
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["upstream-keys"] })
    },
    onError: (e) => {
      console.error("Failed to delete upstream key:", e)
    },
  })

  async function onSave(providerId: string) {
    if (!value.trim()) return
    setKeyMutation.mutate({ providerId, apiKey: value.trim() })
  }

  async function onClear(providerId: string) {
    if (!confirm(t("account.byok.removeConfirm", "Are you sure you want to remove this key?"))) return
    deleteKeyMutation.mutate(providerId)
  }

  function startEditing(providerId: string) {
    setEditing(providerId)
    setValue("")
    setShowKey(false)
  }

  function cancelEditing() {
    setEditing(null)
    setValue("")
    setShowKey(false)
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("account.byok", "Bring Your Own Key")}</CardTitle>
        <CardDescription>
          {t("account.byokDesc", "Use your own API keys instead of the system keys")}
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {isLoading ? (
          <p className="text-muted-foreground">{t("account.loading", "Loading...")}</p>
        ) : (
          providers.map((p) => {
            const hasKey = upstreamKeys.some(k => k.provider_id === p.id)
            const isEditing = editing === p.id
            return (
              <div
                key={p.id}
                className="flex items-center gap-4 py-3 border-b last:border-b-0"
              >
                <div className="w-48">
                  <p className="font-medium">{p.display_name}</p>
                  <p className="text-xs text-muted-foreground">{p.id}</p>
                </div>
                <div className="flex-1">
                  {isEditing ? (
                    <div className="flex gap-2">
                      <div className="relative flex-1">
                        <Input
                          type={showKey ? "text" : "password"}
                          placeholder={t("account.byokPlaceholder", "sk-...")}
                          value={value}
                          onChange={(e) => setValue(e.target.value)}
                          onKeyDown={(e) => {
                            if (e.key === "Enter") onSave(p.id)
                          }}
                          className="pr-10 font-mono"
                        />
                        <Button
                          variant="ghost"
                          size="icon"
                          className="absolute right-1 top-1/2 -translate-y-1/2 h-8 w-8"
                          onClick={() => setShowKey(!showKey)}
                        >
                          {showKey ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                        </Button>
                      </div>
                      <Button onClick={() => onSave(p.id)} disabled={!value.trim() || setKeyMutation.isPending}>
                        <Save className="h-4 w-4 mr-1" />
                        {setKeyMutation.isPending ? t("account.saving", "Saving...") : t("account.save", "Save")}
                      </Button>
                      <Button variant="outline" onClick={cancelEditing}>
                        {t("account.cancel", "Cancel")}
                      </Button>
                    </div>
                  ) : hasKey ? (
                    <p className="text-sm text-muted-foreground">
                      <Badge variant="success" className="mr-2">{t("account.byok.configured", "Configured")}</Badge>
                      Updated: {new Date(upstreamKeys.find(k => k.provider_id === p.id)?.updated_at! * 1000).toLocaleString()}
                    </p>
                  ) : (
                    <p className="text-sm text-muted-foreground">
                      {t("account.byok.usingShared", "Using shared key")}
                      {p.api_key_env?.[0] && (
                        <span className="ml-1 text-xs">(env: {p.api_key_env[0]})</span>
                      )}
                    </p>
                  )}
                </div>
                <div className="flex gap-2">
                  {!isEditing && (
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={() => startEditing(p.id)}
                    >
                      {hasKey ? t("account.byok.replace", "Replace") : t("account.byok.set", "Set")}
                    </Button>
                  )}
                  {hasKey && !isEditing && (
                    <Button
                      size="sm"
                      variant="outline"
                      className="text-destructive"
                      onClick={() => onClear(p.id)}
                      disabled={deleteKeyMutation.isPending}
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  )}
                </div>
              </div>
            )
          })
        )}
      </CardContent>
    </Card>
  )
}

function OAuthSection() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const [githubForm, setGithubForm] = useState({
    clientId: "",
    clientSecret: "",
    callbackUrl: "",
    enabled: false,
  })
  const [giteeForm, setGiteeForm] = useState({
    clientId: "",
    clientSecret: "",
    callbackUrl: "",
    enabled: false,
  })

  // Fetch OAuth clients
  const { data } = useQuery({
    queryKey: ["oauth-clients"],
    queryFn: () => api.account.oauthClients(),
  })

  const clients = data?.clients ?? []

  // Initialize forms from existing clients
  const githubClient = clients.find(c => c.provider === "github")
  const giteeClient = clients.find(c => c.provider === "gitee")

  // Update forms when data loads
  const [initialized, setInitialized] = useState(false)
  if (data && !initialized) {
    setGithubForm({
      clientId: githubClient?.client_id ?? "",
      clientSecret: "",
      callbackUrl: githubClient?.callback_url ?? "",
      enabled: githubClient?.enabled ?? false,
    })
    setGiteeForm({
      clientId: giteeClient?.client_id ?? "",
      clientSecret: "",
      callbackUrl: giteeClient?.callback_url ?? "",
      enabled: giteeClient?.enabled ?? false,
    })
    setInitialized(true)
  }

  const saveGithubMutation = useMutation({
    mutationFn: async () => {
      return api.account.saveOAuthClient("github", {
        clientId: githubForm.clientId,
        clientSecret: githubForm.clientSecret || null,
        callbackUrl: githubForm.callbackUrl,
        enabled: githubForm.enabled,
      })
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["oauth-clients"] })
      setGithubForm(prev => ({ ...prev, clientSecret: "" }))
    },
    onError: (e) => {
      console.error("Failed to save GitHub OAuth:", e)
    },
  })

  const saveGiteeMutation = useMutation({
    mutationFn: async () => {
      return api.account.saveOAuthClient("gitee", {
        clientId: giteeForm.clientId,
        clientSecret: giteeForm.clientSecret || null,
        callbackUrl: giteeForm.callbackUrl,
        enabled: giteeForm.enabled,
      })
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["oauth-clients"] })
      setGiteeForm(prev => ({ ...prev, clientSecret: "" }))
    },
    onError: (e) => {
      console.error("Failed to save Gitee OAuth:", e)
    },
  })

  const deleteGithubMutation = useMutation({
    mutationFn: async () => {
      return api.account.deleteOAuthClient("github")
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["oauth-clients"] })
      setGithubForm({
        clientId: "",
        clientSecret: "",
        callbackUrl: "",
        enabled: false,
      })
    },
    onError: (e) => {
      console.error("Failed to delete GitHub OAuth:", e)
    },
  })

  const deleteGiteeMutation = useMutation({
    mutationFn: async () => {
      return api.account.deleteOAuthClient("gitee")
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["oauth-clients"] })
      setGiteeForm({
        clientId: "",
        clientSecret: "",
        callbackUrl: "",
        enabled: false,
      })
    },
    onError: (e) => {
      console.error("Failed to delete Gitee OAuth:", e)
    },
  })

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("account.oauth", "OAuth Configuration")}</CardTitle>
        <CardDescription>
          {t("account.oauthDesc", "Configure OAuth providers for user authentication")}
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        {/* GitHub OAuth */}
        <div className="border rounded-lg p-4 space-y-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <span className="text-2xl">🐙</span>
              <div>
                <p className="font-medium">GitHub</p>
                {githubClient && (
                  <Badge variant={githubClient.enabled ? "success" : "secondary"} className="mt-1">
                    {githubClient.enabled ? "Enabled" : "Disabled"}
                  </Badge>
                )}
              </div>
            </div>
            <div className="flex gap-2">
              {githubClient && (
                <Button
                  size="sm"
                  variant="outline"
                  className="text-destructive"
                  onClick={() => deleteGithubMutation.mutate()}
                  disabled={deleteGithubMutation.isPending}
                >
                  {t("account.oauth.remove", "Remove")}
                </Button>
              )}
            </div>
          </div>

          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <label className="text-sm font-medium">Client ID</label>
              <Input
                placeholder="GitHub OAuth App Client ID"
                value={githubForm.clientId}
                onChange={(e) => setGithubForm(prev => ({ ...prev, clientId: e.target.value }))}
              />
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">
                Client Secret {githubClient?.has_secret && <span className="text-muted-foreground">(leave blank to keep current)</span>}
              </label>
              <Input
                type="password"
                placeholder={githubClient?.has_secret ? "••••••••" : "GitHub OAuth App Client Secret"}
                value={githubForm.clientSecret}
                onChange={(e) => setGithubForm(prev => ({ ...prev, clientSecret: e.target.value }))}
              />
            </div>
            <div className="space-y-2 md:col-span-2">
              <label className="text-sm font-medium">Callback URL</label>
              <Input
                placeholder="https://your-domain.com/admin/api/auth/oauth/callback/github"
                value={githubForm.callbackUrl}
                onChange={(e) => setGithubForm(prev => ({ ...prev, callbackUrl: e.target.value }))}
              />
            </div>
          </div>

          <div className="flex items-center gap-2">
            <input
              type="checkbox"
              id="github-enabled"
              checked={githubForm.enabled}
              onChange={(e) => setGithubForm(prev => ({ ...prev, enabled: e.target.checked }))}
              className="rounded"
            />
            <label htmlFor="github-enabled" className="text-sm">Enable GitHub OAuth</label>
          </div>

          <div className="flex justify-end">
            <Button
              onClick={() => saveGithubMutation.mutate()}
              disabled={!githubForm.clientId || !githubForm.callbackUrl || saveGithubMutation.isPending}
            >
              {saveGithubMutation.isPending ? t("account.saving", "Saving...") : t("account.save", "Save")}
            </Button>
          </div>
        </div>

        {/* Gitee OAuth */}
        <div className="border rounded-lg p-4 space-y-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <span className="text-2xl">🦊</span>
              <div>
                <p className="font-medium">Gitee</p>
                {giteeClient && (
                  <Badge variant={giteeClient.enabled ? "success" : "secondary"} className="mt-1">
                    {giteeClient.enabled ? "Enabled" : "Disabled"}
                  </Badge>
                )}
              </div>
            </div>
            <div className="flex gap-2">
              {giteeClient && (
                <Button
                  size="sm"
                  variant="outline"
                  className="text-destructive"
                  onClick={() => deleteGiteeMutation.mutate()}
                  disabled={deleteGiteeMutation.isPending}
                >
                  {t("account.oauth.remove", "Remove")}
                </Button>
              )}
            </div>
          </div>

          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <label className="text-sm font-medium">Client ID</label>
              <Input
                placeholder="Gitee OAuth App Client ID"
                value={giteeForm.clientId}
                onChange={(e) => setGiteeForm(prev => ({ ...prev, clientId: e.target.value }))}
              />
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">
                Client Secret {giteeClient?.has_secret && <span className="text-muted-foreground">(leave blank to keep current)</span>}
              </label>
              <Input
                type="password"
                placeholder={giteeClient?.has_secret ? "••••••••" : "Gitee OAuth App Client Secret"}
                value={giteeForm.clientSecret}
                onChange={(e) => setGiteeForm(prev => ({ ...prev, clientSecret: e.target.value }))}
              />
            </div>
            <div className="space-y-2 md:col-span-2">
              <label className="text-sm font-medium">Callback URL</label>
              <Input
                placeholder="https://your-domain.com/admin/api/auth/oauth/callback/gitee"
                value={giteeForm.callbackUrl}
                onChange={(e) => setGiteeForm(prev => ({ ...prev, callbackUrl: e.target.value }))}
              />
            </div>
          </div>

          <div className="flex items-center gap-2">
            <input
              type="checkbox"
              id="gitee-enabled"
              checked={giteeForm.enabled}
              onChange={(e) => setGiteeForm(prev => ({ ...prev, enabled: e.target.checked }))}
              className="rounded"
            />
            <label htmlFor="gitee-enabled" className="text-sm">Enable Gitee OAuth</label>
          </div>

          <div className="flex justify-end">
            <Button
              onClick={() => saveGiteeMutation.mutate()}
              disabled={!giteeForm.clientId || !giteeForm.callbackUrl || saveGiteeMutation.isPending}
            >
              {saveGiteeMutation.isPending ? t("account.saving", "Saving...") : t("account.save", "Save")}
            </Button>
          </div>
        </div>
      </CardContent>
    </Card>
  )
}
