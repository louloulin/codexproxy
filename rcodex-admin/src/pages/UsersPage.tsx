import { useState } from "react"
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { api } from "@/lib/api"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
import { UserPlus, Trash2, Pencil, Shield, User, Loader2, RefreshCw } from "lucide-react"

interface UserItem {
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
}

export function UsersPage() {
  const queryClient = useQueryClient()
  const [error, setError] = useState<string | null>(null)

  // Create dialog state
  const [isCreateOpen, setIsCreateOpen] = useState(false)
  const [newUsername, setNewUsername] = useState("")
  const [newPassword, setNewPassword] = useState("")
  const [newIsAdmin, setNewIsAdmin] = useState(false)

  // Edit dialog state
  const [isEditOpen, setIsEditOpen] = useState(false)
  const [editingUser, setEditingUser] = useState<UserItem | null>(null)
  const [editDisplayName, setEditDisplayName] = useState("")
  const [editEmail, setEditEmail] = useState("")
  const [editIsAdmin, setEditIsAdmin] = useState(false)
  const [editStatus, setEditStatus] = useState("")
  const [editPassword, setEditPassword] = useState("")

  // Fetch users
  const { data: usersData, isLoading, refetch, isFetching } = useQuery({
    queryKey: ["users"],
    queryFn: () => api.users.list(),
  })

  // Create user mutation
  const createUserMutation = useMutation({
    mutationFn: () => api.users.create(newUsername, newPassword, newIsAdmin),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["users"] })
      setIsCreateOpen(false)
      setNewUsername("")
      setNewPassword("")
      setNewIsAdmin(false)
      setError(null)
    },
    onError: (err) => {
      setError(err instanceof Error ? err.message : "Failed to create user")
    },
  })

  // Update user mutation
  const updateUserMutation = useMutation({
    mutationFn: async (data: { id: number; updates: Record<string, unknown> }) => {
      return api.users.update(data.id, data.updates)
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["users"] })
      setIsEditOpen(false)
      setEditingUser(null)
      setEditPassword("")
      setError(null)
    },
    onError: (err) => {
      setError(err instanceof Error ? err.message : "Failed to update user")
    },
  })

  // Delete user mutation
  const deleteUserMutation = useMutation({
    mutationFn: (id: number) => api.users.delete(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["users"] })
      setError(null)
    },
    onError: (err) => {
      setError(err instanceof Error ? err.message : "Failed to delete user")
    },
  })

  const users: UserItem[] = usersData?.users || []

  function handleCreateUser(e: React.FormEvent) {
    e.preventDefault()
    if (!newUsername || !newPassword) {
      setError("Username and password are required")
      return
    }
    if (newPassword.length < 8) {
      setError("Password must be at least 8 characters")
      return
    }
    createUserMutation.mutate()
  }

  function openEditDialog(user: UserItem) {
    setEditingUser(user)
    setEditDisplayName(user.displayName || "")
    setEditEmail(user.email || "")
    setEditIsAdmin(user.isAdmin)
    setEditStatus(user.status)
    setEditPassword("")
    setIsEditOpen(true)
  }

  function handleUpdateUser(e: React.FormEvent) {
    e.preventDefault()
    if (!editingUser) return

    const updates: Record<string, unknown> = {}

    // Only include changed fields
    if (editDisplayName !== (editingUser.displayName || "")) {
      updates.display_name = editDisplayName || null
    }
    if (editEmail !== (editingUser.email || "")) {
      updates.email = editEmail || null
    }
    if (editIsAdmin !== editingUser.isAdmin) {
      updates.is_admin = editIsAdmin
    }
    if (editStatus !== editingUser.status) {
      updates.status = editStatus
    }
    if (editPassword) {
      if (editPassword.length < 8) {
        setError("Password must be at least 8 characters")
        return
      }
      updates.password = editPassword
    }

    // If no changes, just close the dialog
    if (Object.keys(updates).length === 0) {
      setIsEditOpen(false)
      setEditingUser(null)
      return
    }

    updateUserMutation.mutate({ id: editingUser.id, updates })
  }

  function handleDeleteUser(id: number) {
    if (confirm("Are you sure you want to delete this user?")) {
      deleteUserMutation.mutate(id)
    }
  }

  function resetCreateForm() {
    setNewUsername("")
    setNewPassword("")
    setNewIsAdmin(false)
    setError(null)
  }

  return (
    <div className="container mx-auto py-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">Users</h1>
          <p className="text-muted-foreground">Manage user accounts</p>
        </div>

        <div className="flex gap-2">
          <Button variant="outline" onClick={() => refetch()} disabled={isFetching}>
            <RefreshCw className={`mr-2 h-4 w-4 ${isFetching ? "animate-spin" : ""}`} />
            Refresh
          </Button>

          <Dialog open={isCreateOpen} onOpenChange={(open) => {
            setIsCreateOpen(open)
            if (!open) resetCreateForm()
          }}>
            <DialogTrigger asChild>
              <Button>
                <UserPlus className="mr-2 h-4 w-4" />
                Add User
              </Button>
            </DialogTrigger>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>Create New User</DialogTitle>
                <DialogDescription>Add a new user account to the system</DialogDescription>
              </DialogHeader>
              <form onSubmit={handleCreateUser}>
                {error && (
                  <Alert variant="destructive" className="mb-4">
                    <AlertDescription>{error}</AlertDescription>
                  </Alert>
                )}
                <div className="space-y-4 py-4">
                  <div className="space-y-2">
                    <Label htmlFor="username">Username</Label>
                    <Input
                      id="username"
                      value={newUsername}
                      onChange={(e) => setNewUsername(e.target.value)}
                      placeholder="username"
                      required
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="password">Password</Label>
                    <Input
                      id="password"
                      type="password"
                      value={newPassword}
                      onChange={(e) => setNewPassword(e.target.value)}
                      placeholder="password (min 8 characters)"
                      required
                      minLength={8}
                    />
                  </div>
                  <div className="flex items-center space-x-2">
                    <input
                      type="checkbox"
                      id="isAdmin"
                      checked={newIsAdmin}
                      onChange={(e) => setNewIsAdmin(e.target.checked)}
                      className="rounded border-gray-300"
                    />
                    <Label htmlFor="isAdmin">Admin user</Label>
                  </div>
                </div>
                <DialogFooter>
                  <Button type="button" variant="outline" onClick={() => setIsCreateOpen(false)}>
                    Cancel
                  </Button>
                  <Button type="submit" disabled={createUserMutation.isPending}>
                    {createUserMutation.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                    Create User
                  </Button>
                </DialogFooter>
              </form>
            </DialogContent>
          </Dialog>
        </div>
      </div>

      {error && (
        <Alert variant="destructive">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      <Card>
        <CardHeader>
          <CardTitle>User Accounts</CardTitle>
          <CardDescription>{users.length} user(s) in the system</CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>ID</TableHead>
                  <TableHead>Username</TableHead>
                  <TableHead>Display Name</TableHead>
                  <TableHead>Email</TableHead>
                  <TableHead>Role</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Created</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {users.length === 0 ? (
                  <TableRow>
                    <TableCell colSpan={8} className="text-center py-8 text-muted-foreground">
                      No users found
                    </TableCell>
                  </TableRow>
                ) : (
                  users.map((user) => (
                    <TableRow key={user.id}>
                      <TableCell className="font-mono text-sm">{user.id}</TableCell>
                      <TableCell className="font-medium">
                        <div className="flex items-center gap-2">
                          {user.isAdmin ? (
                            <Shield className="h-4 w-4 text-primary" />
                          ) : (
                            <User className="h-4 w-4 text-muted-foreground" />
                          )}
                          {user.username}
                        </div>
                      </TableCell>
                      <TableCell>{user.displayName || "-"}</TableCell>
                      <TableCell>{user.email || "-"}</TableCell>
                      <TableCell>
                        {user.isAdmin ? (
                          <Badge variant="default">Admin</Badge>
                        ) : (
                          <Badge variant="secondary">User</Badge>
                        )}
                      </TableCell>
                      <TableCell>
                        <Badge variant={user.status === "active" ? "default" : "destructive"}>
                          {user.status}
                        </Badge>
                      </TableCell>
                      <TableCell className="text-muted-foreground">
                        {new Date(user.createdAt).toLocaleDateString()}
                      </TableCell>
                      <TableCell className="text-right">
                        <div className="flex justify-end gap-1">
                          <Button
                            variant="ghost"
                            size="icon"
                            onClick={() => openEditDialog(user)}
                          >
                            <Pencil className="h-4 w-4" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon"
                            onClick={() => handleDeleteUser(user.id)}
                            disabled={deleteUserMutation.isPending}
                            className="text-destructive hover:text-destructive"
                          >
                            <Trash2 className="h-4 w-4" />
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>
                  ))
                )}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Edit User Dialog */}
      <Dialog open={isEditOpen} onOpenChange={(open) => {
        setIsEditOpen(open)
        if (!open) {
          setEditingUser(null)
          setEditPassword("")
        }
      }}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Edit User: {editingUser?.username}</DialogTitle>
            <DialogDescription>Update user account information</DialogDescription>
          </DialogHeader>
          <form onSubmit={handleUpdateUser}>
            {error && (
              <Alert variant="destructive" className="mb-4">
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}
            <div className="space-y-4 py-4">
              <div className="space-y-2">
                <Label htmlFor="editDisplayName">Display Name</Label>
                <Input
                  id="editDisplayName"
                  value={editDisplayName}
                  onChange={(e) => setEditDisplayName(e.target.value)}
                  placeholder="Display name (optional)"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="editEmail">Email</Label>
                <Input
                  id="editEmail"
                  type="email"
                  value={editEmail}
                  onChange={(e) => setEditEmail(e.target.value)}
                  placeholder="Email (optional)"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="editPassword">New Password</Label>
                <Input
                  id="editPassword"
                  type="password"
                  value={editPassword}
                  onChange={(e) => setEditPassword(e.target.value)}
                  placeholder="Leave blank to keep current password (min 8 chars)"
                  minLength={8}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="editStatus">Status</Label>
                <select
                  id="editStatus"
                  value={editStatus}
                  onChange={(e) => setEditStatus(e.target.value)}
                  className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
                >
                  <option value="active">Active</option>
                  <option value="disabled">Disabled</option>
                </select>
              </div>
              <div className="flex items-center space-x-2">
                <input
                  type="checkbox"
                  id="editIsAdmin"
                  checked={editIsAdmin}
                  onChange={(e) => setEditIsAdmin(e.target.checked)}
                  className="rounded border-gray-300"
                />
                <Label htmlFor="editIsAdmin">Admin user</Label>
              </div>
            </div>
            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => setIsEditOpen(false)}>
                Cancel
              </Button>
              <Button type="submit" disabled={updateUserMutation.isPending}>
                {updateUserMutation.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                Update User
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </div>
  )
}
