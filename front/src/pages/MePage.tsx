import { useEffect, useState, useRef } from 'react'
import { Navigate } from 'react-router-dom'
// eslint-disable-next-line @typescript-eslint/no-deprecated
import { Camera, Mail, Phone, Calendar, Shield, User, Loader2, Trophy, Zap, Flame, Award, Star, Target, Settings, Bot, CheckCircle, XCircle, Play, Link2, Unlink2, KeyRound, Github, Chrome } from 'lucide-react'

import { Card, CardContent } from '../components/ui/card'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../components/ui/dialog'
import { Button } from '../components/ui/button'
import { Input } from '../components/ui/input'
import { Label } from '../components/ui/label'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../components/ui/tabs'
import { Footer } from '../components/Footer'
import { Header } from '../components/Header'
import { AvatarCropDialog } from '../components/AvatarCropDialog'
import {
  me, updateMe, uploadAvatar, deleteAvatar, fetchAvatarUrl,
  getUserProfileSummary, getUserAchievements,
  listAiProviders, createAiProvider, updateAiProvider, deleteAiProvider, testAiProvider, activateAiProvider,
  getAccountStatus, unlinkOAuthIdentity, linkOAuthIdentity, changePassword,
  type UserProfileSummary, type AchievementWithProgress, type UserAiProvider, type AiProviderType, type AccountStatus
} from '../lib/api'
import { OAUTH_PROVIDERS, type OAuthProviderKey } from '../lib/oauth-config'
import { cn } from '../lib/utils'
import { useAuth } from '../lib/auth'
import { useToast } from '../contexts/ToastContext'

function formatDate(value?: string | null) {
  if (!value) return '-'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return date.toLocaleDateString('zh-CN', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  })
}

export function MePage() {
  const { token, user, setAuth } = useAuth()
  const toast = useToast()
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // Profile form state
  const [name, setName] = useState(user?.name || '')
  const [phone, setPhone] = useState(user?.phone || '')
  const [saving, setSaving] = useState(false)
  const [saveError, setSaveError] = useState<string | null>(null)
  const [saved, setSaved] = useState(false)

  // Avatar state
  const [avatarUploading, setAvatarUploading] = useState(false)
  const [avatarError, setAvatarError] = useState<string | null>(null)
  const [avatarUrl, setAvatarUrl] = useState<string | null>(null)
  const [cropDialogImage, setCropDialogImage] = useState<string | null>(null)
  const fileInputRef = useRef<HTMLInputElement>(null)

  // Achievement state
  const [profile, setProfile] = useState<UserProfileSummary | null>(null)
  const [achievements, setAchievements] = useState<AchievementWithProgress[]>([])
  const [achievementsLoading, setAchievementsLoading] = useState(false)

  // AI Provider state
  const [aiProviders, setAiProviders] = useState<UserAiProvider[]>([])
  const [aiProvidersLoading, setAiProvidersLoading] = useState(false)
  const [selectedProviderType, setSelectedProviderType] = useState<AiProviderType | null>(null)
  const [providerFormData, setProviderFormData] = useState<Record<string, string>>({})
  const [providerSaving, setProviderSaving] = useState(false)
  const [providerTesting, setProviderTesting] = useState<number | null>(null)

  // Account status state
  const [accountStatus, setAccountStatus] = useState<AccountStatus | null>(null)
  const [accountStatusLoading, setAccountStatusLoading] = useState(false)
  const [unlinkingId, setUnlinkingId] = useState<number | null>(null)
  const [linkingProvider, setLinkingProvider] = useState<string | null>(null)

  // Password change dialog state
  const [passwordDialogOpen, setPasswordDialogOpen] = useState(false)
  const [currentPassword, setCurrentPassword] = useState('')
  const [newPassword, setNewPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [passwordSaving, setPasswordSaving] = useState(false)
  const [passwordError, setPasswordError] = useState<string | null>(null)

  useEffect(() => {
    if (!token) return
    let cancelled = false
    setLoading(true)
    setError(null)

    me(token)
      .then((profile) => {
        if (cancelled) return
        setAuth(token, profile)
      })
      .catch((e: unknown) => {
        if (cancelled) return
        setError(e instanceof Error ? e.message : 'Failed to load profile')
      })
      .finally(() => {
        if (cancelled) return
        setLoading(false)
      })

    return () => {
      cancelled = true
    }
  }, [token, setAuth])

  useEffect(() => {
    setName(user?.name || '')
    setPhone(user?.phone || '')
  }, [user?.name, user?.phone])

  // Fetch avatar URL when user has avatar
  useEffect(() => {
    if (!token || !user?.avatar) {
      setAvatarUrl(null)
      return
    }

    let cancelled = false
    fetchAvatarUrl(token).then((url) => {
      if (!cancelled) {
        setAvatarUrl(url)
      }
    })

    return () => {
      cancelled = true
    }
  }, [token, user?.avatar])

  // Fetch achievements data
  useEffect(() => {
    if (!token) return
    let cancelled = false
    setAchievementsLoading(true)

    Promise.all([
      getUserProfileSummary(token),
      getUserAchievements(token),
    ])
      .then(([profileData, achievementsData]) => {
        if (cancelled) return
        setProfile(profileData)
        setAchievements(achievementsData)
      })
      .catch(() => {
        if (cancelled) return
        setProfile(null)
        setAchievements([])
      })
      .finally(() => {
        if (cancelled) return
        setAchievementsLoading(false)
      })

    return () => {
      cancelled = true
    }
  }, [token])

  // Fetch AI providers
  useEffect(() => {
    if (!token) return
    let cancelled = false
    setAiProvidersLoading(true)

    listAiProviders(token)
      .then((providers) => {
        if (cancelled) return
        setAiProviders(providers)
      })
      .catch(() => {
        if (cancelled) return
        setAiProviders([])
      })
      .finally(() => {
        if (cancelled) return
        setAiProvidersLoading(false)
      })

    return () => {
      cancelled = true
    }
  }, [token])

  // Fetch account status
  useEffect(() => {
    if (!token) return
    let cancelled = false
    setAccountStatusLoading(true)

    getAccountStatus(token)
      .then((status) => {
        if (cancelled) return
        setAccountStatus(status)
      })
      .catch(() => {
        if (cancelled) return
        setAccountStatus(null)
      })
      .finally(() => {
        if (cancelled) return
        setAccountStatusLoading(false)
      })

    return () => {
      cancelled = true
    }
  }, [token])

  const handleUnlinkIdentity = async (identityId: number) => {
    if (!token) return

    setUnlinkingId(identityId)
    try {
      await unlinkOAuthIdentity(token, identityId)
      // Refresh account status
      const status = await getAccountStatus(token)
      setAccountStatus(status)
      toast.success('解绑成功', '社交账号已解除绑定')
    } catch (e) {
      const message = e instanceof Error ? e.message : '解绑失败'
      toast.error('解绑失败', message)
    } finally {
      setUnlinkingId(null)
    }
  }

  const handleLinkIdentity = async (provider: string) => {
    if (!token) return

    const providerName = OAUTH_PROVIDERS[provider as OAuthProviderKey]?.name || provider

    // Simulate OAuth flow with prompts (same as login flow)
    // In production, this would redirect to the OAuth provider's authorization page
    const userId = prompt(`请输入您的 ${providerName} 用户 ID：`)
    if (!userId) return

    const email = prompt(`请输入您的 ${providerName} 邮箱（可选）：`)

    setLinkingProvider(provider)
    try {
      await linkOAuthIdentity(token, {
        provider,
        provider_user_id: userId,
        email: email || undefined,
      })
      // Refresh account status
      const status = await getAccountStatus(token)
      setAccountStatus(status)
      toast.success('绑定成功', `已成功绑定 ${providerName} 账号`)
    } catch (e) {
      const message = e instanceof Error ? e.message : '绑定失败'
      toast.error('绑定失败', message)
    } finally {
      setLinkingProvider(null)
    }
  }

  const handleChangePassword = async () => {
    if (!token) return
    setPasswordError(null)

    if (newPassword.length < 8) {
      setPasswordError('密码长度至少 8 个字符')
      return
    }
    if (newPassword !== confirmPassword) {
      setPasswordError('两次输入的密码不一致')
      return
    }

    setPasswordSaving(true)
    try {
      await changePassword(
        token,
        newPassword,
        accountStatus?.has_password ? currentPassword : undefined,
      )
      // Refresh account status
      const status = await getAccountStatus(token)
      setAccountStatus(status)
      // Reset form and close dialog
      setPasswordDialogOpen(false)
      setCurrentPassword('')
      setNewPassword('')
      setConfirmPassword('')
      toast.success('密码修改成功', accountStatus?.has_password ? '密码已更新' : '密码已设置')
    } catch (e) {
      setPasswordError(e instanceof Error ? e.message : '密码修改失败')
    } finally {
      setPasswordSaving(false)
    }
  }

  const saveProfile = async () => {
    if (!token || !user) return
    setSaving(true)
    setSaveError(null)
    setSaved(false)
    try {
      const next = await updateMe(token, {
        name: name || null,
        phone: phone || null,
      })
      setAuth(token, next)
      setSaved(true)
      setTimeout(() => setSaved(false), 3000)
    } catch (e: unknown) {
      setSaveError(e instanceof Error ? e.message : 'Failed to save')
    } finally {
      setSaving(false)
    }
  }

  const handleAvatarClick = () => {
    fileInputRef.current?.click()
  }

  const handleAvatarChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (!file || !token) return

    if (!file.type.startsWith('image/')) {
      setAvatarError('请选择图片文件')
      return
    }

    if (file.size > 10 * 1024 * 1024) {
      setAvatarError('图片大小不能超过 10MB')
      return
    }

    setAvatarError(null)
    const imageUrl = URL.createObjectURL(file)
    setCropDialogImage(imageUrl)

    if (fileInputRef.current) {
      fileInputRef.current.value = ''
    }
  }

  const handleCroppedImage = async (croppedBlob: Blob) => {
    if (!token) return

    if (cropDialogImage) {
      URL.revokeObjectURL(cropDialogImage)
    }
    setCropDialogImage(null)

    setAvatarUploading(true)
    setAvatarError(null)

    try {
      const croppedFile = new File([croppedBlob], 'avatar.jpg', { type: 'image/jpeg' })
      const updatedUser = await uploadAvatar(token, croppedFile)
      setAuth(token, updatedUser)
    } catch (e: unknown) {
      setAvatarError(e instanceof Error ? e.message : '上传失败')
    } finally {
      setAvatarUploading(false)
    }
  }

  const handleCropCancel = () => {
    if (cropDialogImage) {
      URL.revokeObjectURL(cropDialogImage)
    }
    setCropDialogImage(null)
  }

  const handleDeleteAvatar = async () => {
    if (!token || !user?.avatar) return

    setAvatarUploading(true)
    setAvatarError(null)

    try {
      const updatedUser = await deleteAvatar(token)
      setAuth(token, updatedUser)
    } catch (e: unknown) {
      setAvatarError(e instanceof Error ? e.message : '删除失败')
    } finally {
      setAvatarUploading(false)
    }
  }

  // AI Provider handlers
  const handleSaveProvider = async () => {
    if (!token || !selectedProviderType) return

    setProviderSaving(true)
    try {
      const existingProvider = aiProviders.find(p => p.provider_type === selectedProviderType)

      if (existingProvider) {
        const updated = await updateAiProvider(token, existingProvider.id, {
          settings: providerFormData,
        })
        setAiProviders(prev => prev.map(p => p.id === updated.id ? updated : p))
      } else {
        const created = await createAiProvider(token, {
          provider_type: selectedProviderType,
          settings: providerFormData,
          set_active: aiProviders.length === 0,
        })
        setAiProviders(prev => [...prev, created])
      }
      setSelectedProviderType(null)
      setProviderFormData({})
      toast.success('保存成功', '配置已保存')
    } catch (e) {
      const message = e instanceof Error ? e.message : '保存失败'
      toast.error('保存失败', message)
    } finally {
      setProviderSaving(false)
    }
  }

  const handleTestProvider = async (providerId: number) => {
    if (!token) return

    setProviderTesting(providerId)
    try {
      const result = await testAiProvider(token, providerId)
      // Update the provider with test result
      setAiProviders(prev => prev.map(p =>
        p.id === providerId
          ? { ...p, test_status: result.success ? 'success' : 'failed', test_error: result.error, last_tested_at: result.tested_at }
          : p
      ))
      if (result.success) {
        toast.success('测试成功', 'AI 提供商连接正常')
      } else {
        toast.error('测试失败', result.error || '连接失败，请检查配置')
      }
    } catch (e) {
      const message = e instanceof Error ? e.message : '测试失败'
      toast.error('测试失败', message)
    } finally {
      setProviderTesting(null)
    }
  }

  const handleActivateProvider = async (providerId: number) => {
    if (!token) return

    try {
      const activated = await activateAiProvider(token, providerId)
      setAiProviders(prev => prev.map(p => ({
        ...p,
        is_active: p.id === activated.id,
      })))
      toast.success('激活成功', '已切换 AI 提供商')
    } catch (e) {
      const message = e instanceof Error ? e.message : '激活失败'
      toast.error('激活失败', message)
    }
  }

  const handleDeleteProvider = async (providerId: number) => {
    if (!token) return

    try {
      await deleteAiProvider(token, providerId)
      setAiProviders(prev => prev.filter(p => p.id !== providerId))
      toast.success('删除成功', '配置已删除')
    } catch (e) {
      const message = e instanceof Error ? e.message : '删除失败'
      toast.error('删除失败', message)
    }
  }

  if (!token) {
    return <Navigate to="/" replace />
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-50 via-gray-50 to-zinc-100">
      <Header />

      <main className="mx-auto max-w-6xl px-4 py-8">
        {/* Profile Header Card */}
        <Card className="mb-6 overflow-hidden border-0 shadow-xl">
          <div className="h-24 bg-gradient-to-r from-orange-500 via-amber-500 to-orange-400" />
          <CardContent className="relative pb-4">
            <div className="flex flex-col sm:flex-row sm:items-end gap-4 -mt-12">
              {/* Avatar */}
              <div className="relative group">
                <div className="h-24 w-24 rounded-2xl bg-white p-1 shadow-xl">
                  <div className="h-full w-full rounded-xl bg-gradient-to-br from-orange-500 to-amber-400 flex items-center justify-center overflow-hidden">
                    {avatarUrl ? (
                      <img src={avatarUrl} alt="Avatar" className="h-full w-full object-cover" />
                    ) : (
                      <User className="h-12 w-12 text-white" />
                    )}
                  </div>
                </div>
                <button
                  onClick={handleAvatarClick}
                  disabled={avatarUploading}
                  className="absolute inset-1 rounded-xl bg-black/50 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center cursor-pointer disabled:cursor-not-allowed"
                >
                  {avatarUploading ? (
                    <Loader2 className="h-6 w-6 text-white animate-spin" />
                  ) : (
                    <Camera className="h-6 w-6 text-white" />
                  )}
                </button>
                <input
                  ref={fileInputRef}
                  type="file"
                  accept="image/*"
                  onChange={handleAvatarChange}
                  className="hidden"
                />
              </div>

              {/* User Info */}
              <div className="flex-1 pt-2 sm:pt-0 sm:pb-1">
                <h1 className="text-xl font-bold text-gray-900">
                  {user?.name || user?.email?.split('@')[0] || '用户'}
                </h1>
                <p className="text-sm text-gray-500">{user?.email}</p>
                {avatarError && <p className="text-sm text-red-500 mt-1">{avatarError}</p>}
              </div>

              {user?.avatar && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleDeleteAvatar}
                  disabled={avatarUploading}
                  className="text-red-600 hover:text-red-700 hover:bg-red-50"
                >
                  删除头像
                </Button>
              )}
            </div>
          </CardContent>
        </Card>

        {/* Tabs */}
        <Tabs defaultValue="profile" className="space-y-6">
          <TabsList className="grid w-full grid-cols-3">
            <TabsTrigger value="profile" className="gap-2">
              <User className="h-4 w-4" />
              <span className="hidden sm:inline">个人资料</span>
            </TabsTrigger>
            <TabsTrigger value="achievements" className="gap-2">
              <Trophy className="h-4 w-4" />
              <span className="hidden sm:inline">成就</span>
            </TabsTrigger>
            <TabsTrigger value="settings" className="gap-2">
              <Settings className="h-4 w-4" />
              <span className="hidden sm:inline">设置</span>
            </TabsTrigger>
          </TabsList>

          {/* Profile Tab */}
          <TabsContent value="profile">
            <div className="grid gap-6 md:grid-cols-2">
              {/* Account Information */}
              <Card className="border-0 shadow-lg">
                <CardContent className="p-6">
                  <div className="flex items-center gap-3 mb-4">
                    <div className="h-8 w-8 rounded-lg bg-blue-100 flex items-center justify-center">
                      <Shield className="h-4 w-4 text-blue-600" />
                    </div>
                    <h2 className="text-base font-semibold text-gray-900">账户信息</h2>
                  </div>

                  <div className="space-y-3">
                    <div className="flex items-center gap-3 p-2 rounded-lg bg-gray-50">
                      <Mail className="h-4 w-4 text-gray-400" />
                      <div className="flex-1 min-w-0">
                        <p className="text-xs text-gray-500">邮箱地址</p>
                        <p className="text-sm font-medium text-gray-900 truncate">{user?.email || '-'}</p>
                      </div>
                    </div>

                    <div className="flex items-center gap-3 p-2 rounded-lg bg-gray-50">
                      <Phone className="h-4 w-4 text-gray-400" />
                      <div className="flex-1 min-w-0">
                        <p className="text-xs text-gray-500">手机号码</p>
                        <p className="text-sm font-medium text-gray-900">{user?.phone || '未设置'}</p>
                      </div>
                    </div>

                    <div className="flex items-center gap-3 p-2 rounded-lg bg-gray-50">
                      <Calendar className="h-4 w-4 text-gray-400" />
                      <div className="flex-1 min-w-0">
                        <p className="text-xs text-gray-500">注册时间</p>
                        <p className="text-sm font-medium text-gray-900">{formatDate(user?.created_at)}</p>
                      </div>
                    </div>
                  </div>

                  {loading && (
                    <div className="flex items-center gap-2 mt-4 text-sm text-gray-500">
                      <Loader2 className="h-4 w-4 animate-spin" />
                      加载中...
                    </div>
                  )}
                  {error && <p className="text-sm text-red-500 mt-4">{error}</p>}
                </CardContent>
              </Card>

              {/* Edit Profile */}
              <Card className="border-0 shadow-lg">
                <CardContent className="p-6">
                  <div className="flex items-center gap-3 mb-4">
                    <div className="h-8 w-8 rounded-lg bg-orange-100 flex items-center justify-center">
                      <User className="h-4 w-4 text-orange-600" />
                    </div>
                    <h2 className="text-base font-semibold text-gray-900">编辑资料</h2>
                  </div>

                  <div className="space-y-4">
                    <div className="space-y-2">
                      <Label htmlFor="edit-name" className="text-sm font-medium">显示名称</Label>
                      <Input
                        id="edit-name"
                        value={name}
                        onChange={(e) => setName(e.target.value)}
                        placeholder="输入您的姓名"
                        className="h-9"
                      />
                    </div>

                    <Button
                      onClick={saveProfile}
                      disabled={saving}
                      className="w-full bg-gradient-to-r from-orange-500 to-amber-500 hover:from-orange-600 hover:to-amber-600"
                    >
                      {saving ? (
                        <>
                          <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                          保存中...
                        </>
                      ) : (
                        '保存更改'
                      )}
                    </Button>

                    {saved && <p className="text-sm text-green-600 text-center">资料更新成功！</p>}
                    {saveError && <p className="text-sm text-red-500 text-center">{saveError}</p>}
                  </div>
                </CardContent>
              </Card>
            </div>

            {/* Account Status & Linked Accounts */}
            <Card className="border-0 shadow-lg mt-6">
              <CardContent className="p-6">
                <div className="flex items-center gap-3 mb-4">
                  <div className="h-8 w-8 rounded-lg bg-purple-100 flex items-center justify-center">
                    <Link2 className="h-4 w-4 text-purple-600" />
                  </div>
                  <div>
                    <h2 className="text-base font-semibold text-gray-900">账户状态</h2>
                    <p className="text-xs text-gray-500">管理您的登录方式</p>
                  </div>
                </div>

                {accountStatusLoading ? (
                  <div className="flex items-center justify-center py-8">
                    <Loader2 className="h-6 w-6 animate-spin text-purple-500" />
                  </div>
                ) : accountStatus ? (
                  <div className="space-y-4">
                    {/* Login Method Status */}
                    <div className="p-3 rounded-lg bg-gray-50">
                      <div className="flex items-center gap-3">
                        <div className={cn(
                          "h-8 w-8 rounded-lg flex items-center justify-center",
                          accountStatus.has_password ? "bg-green-100" : "bg-gray-200"
                        )}>
                          <KeyRound className={cn(
                            "h-4 w-4",
                            accountStatus.has_password ? "text-green-600" : "text-gray-400"
                          )} />
                        </div>
                        <div className="flex-1">
                          <p className="text-sm font-medium text-gray-900">邮箱密码登录</p>
                          <p className="text-xs text-gray-500">
                            {accountStatus.has_password ? '已设置密码' : '未设置密码（仅通过社交账号登录）'}
                          </p>
                        </div>
                        <Button
                          variant="outline"
                          size="sm"
                          className="text-xs"
                          onClick={() => setPasswordDialogOpen(true)}
                        >
                          {accountStatus.has_password ? '修改密码' : '设置密码'}
                        </Button>
                      </div>
                    </div>

                    {/* Linked Social Accounts */}
                    <div className="space-y-2">
                      <p className="text-sm font-medium text-gray-700">已绑定的社交账号</p>

                      {accountStatus.linked_identities.length > 0 ? (
                        <div className="space-y-2">
                          {accountStatus.linked_identities.map((identity) => {
                            const providerConfig = OAUTH_PROVIDERS[identity.provider as OAuthProviderKey]
                            const providerName = providerConfig?.name || identity.provider

                            return (
                              <div
                                key={identity.id}
                                className="flex items-center gap-3 p-3 rounded-lg border border-gray-200 bg-white"
                              >
                                <div className="h-8 w-8 rounded-lg bg-gray-100 flex items-center justify-center">
                                  {identity.provider === 'github' ? (
                                    <Github className="h-4 w-4 text-gray-700" />
                                  ) : identity.provider === 'google' ? (
                                    <Chrome className="h-4 w-4 text-gray-700" />
                                  ) : (
                                    <User className="h-4 w-4 text-gray-500" />
                                  )}
                                </div>
                                <div className="flex-1 min-w-0">
                                  <p className="text-sm font-medium text-gray-900">{providerName}</p>
                                  {identity.email && (
                                    <p className="text-xs text-gray-500 truncate">{identity.email}</p>
                                  )}
                                </div>
                                <Button
                                  variant="outline"
                                  size="sm"
                                  className="h-7 text-xs text-red-600 hover:text-red-700 hover:bg-red-50"
                                  onClick={() => handleUnlinkIdentity(identity.id)}
                                  disabled={unlinkingId === identity.id || (!accountStatus.has_password && accountStatus.linked_identities.length <= 1)}
                                >
                                  {unlinkingId === identity.id ? (
                                    <Loader2 className="h-3 w-3 animate-spin" />
                                  ) : (
                                    <>
                                      <Unlink2 className="h-3 w-3 mr-1" />
                                      解绑
                                    </>
                                  )}
                                </Button>
                              </div>
                            )
                          })}
                        </div>
                      ) : (
                        <div className="text-center py-4 text-gray-500 text-sm">
                          暂未绑定任何社交账号
                        </div>
                      )}
                    </div>

                    {/* Available Providers to Link */}
                    <div className="space-y-2">
                      <p className="text-sm font-medium text-gray-700">可绑定的社交账号</p>
                      <div className="grid gap-2 sm:grid-cols-2">
                        {Object.entries(OAUTH_PROVIDERS).map(([key, config]) => {
                          if (!config.enabled) return null
                          const isLinked = accountStatus.linked_identities.some(i => i.provider === key)
                          if (isLinked) return null

                          const isLinking = linkingProvider === key

                          return (
                            <button
                              key={key}
                              onClick={() => handleLinkIdentity(key)}
                              disabled={isLinking || linkingProvider !== null}
                              className="flex items-center gap-3 p-3 rounded-lg border border-dashed border-gray-300 bg-gray-50 hover:bg-gray-100 hover:border-purple-300 transition-colors cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed text-left"
                            >
                              <div className="h-8 w-8 rounded-lg bg-gray-200 flex items-center justify-center">
                                {isLinking ? (
                                  <Loader2 className="h-4 w-4 text-purple-500 animate-spin" />
                                ) : key === 'github' ? (
                                  <Github className="h-4 w-4 text-gray-500" />
                                ) : key === 'google' ? (
                                  <Chrome className="h-4 w-4 text-gray-500" />
                                ) : (
                                  <User className="h-4 w-4 text-gray-400" />
                                )}
                              </div>
                              <div className="flex-1">
                                <p className="text-sm text-gray-600">{config.name}</p>
                                <p className="text-xs text-gray-400">
                                  {isLinking ? '绑定中...' : '点击绑定'}
                                </p>
                              </div>
                              <Link2 className="h-4 w-4 text-gray-400" />
                            </button>
                          )
                        })}
                      </div>
                    </div>
                  </div>
                ) : (
                  <div className="text-center py-8 text-gray-500">
                    <Link2 className="h-8 w-8 mx-auto mb-2 text-gray-300" />
                    <p className="text-sm">无法加载账户状态</p>
                  </div>
                )}
              </CardContent>
            </Card>
          </TabsContent>

          {/* Achievements Tab */}
          <TabsContent value="achievements">
            {achievementsLoading ? (
              <div className="flex items-center justify-center py-12">
                <Loader2 className="h-8 w-8 animate-spin text-orange-500" />
              </div>
            ) : profile ? (
              <div className="space-y-6">
                {/* Rank & XP Card */}
                <Card className="border-0 shadow-lg overflow-hidden">
                  <div className="bg-gradient-to-r from-orange-500 via-amber-500 to-orange-400 p-6">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-4">
                        <div
                          className="h-14 w-14 rounded-full flex items-center justify-center shadow-lg"
                          style={{ backgroundColor: profile.rank?.color || '#9CA3AF' }}
                        >
                          <Trophy className="h-7 w-7 text-white" />
                        </div>
                        <div>
                          <p className="text-xl font-bold text-white">{profile.rank?.name_zh || '新手'}</p>
                          <p className="text-white/80">Lv.{profile.rank?.level || 1}</p>
                        </div>
                      </div>
                      <div className="text-right">
                        <div className="flex items-center gap-2">
                          <Zap className="h-5 w-5 text-yellow-200" />
                          <span className="text-2xl font-bold text-white">{profile.total_xp}</span>
                          <span className="text-white/80">XP</span>
                        </div>
                        {profile.current_streak_days > 0 && (
                          <div className="flex items-center gap-1 mt-1 justify-end">
                            <Flame className="h-4 w-4 text-yellow-200" />
                            <span className="text-sm text-white/90 font-medium">
                              {profile.current_streak_days}天连续学习
                            </span>
                          </div>
                        )}
                      </div>
                    </div>

                    {profile.next_rank && (
                      <div className="mt-4 space-y-2">
                        <div className="flex justify-between text-sm text-white/80">
                          <span>距离 {profile.next_rank.name_zh}</span>
                          <span>还需 {profile.xp_to_next_rank} XP</span>
                        </div>
                        <div className="h-2 bg-white/20 rounded-full overflow-hidden">
                          <div
                            className="h-full bg-white rounded-full transition-all"
                            style={{
                              width: `${Math.min(100, ((profile.total_xp - (profile.rank?.min_xp || 0)) / (profile.next_rank.min_xp - (profile.rank?.min_xp || 0))) * 100)}%`
                            }}
                          />
                        </div>
                      </div>
                    )}
                  </div>

                  <CardContent className="p-4">
                    <div className="grid grid-cols-3 gap-3">
                      <div className="text-center p-3 bg-orange-50 rounded-xl">
                        <Flame className="h-5 w-5 text-orange-500 mx-auto mb-1" />
                        <p className="text-xl font-bold text-orange-600">{profile.current_streak_days}</p>
                        <p className="text-xs text-gray-500">连续天数</p>
                      </div>
                      <div className="text-center p-3 bg-green-50 rounded-xl">
                        <Award className="h-5 w-5 text-green-500 mx-auto mb-1" />
                        <p className="text-xl font-bold text-green-600">{profile.completed_achievements}</p>
                        <p className="text-xs text-gray-500">已获成就</p>
                      </div>
                      <div className="text-center p-3 bg-blue-50 rounded-xl">
                        <Trophy className="h-5 w-5 text-blue-500 mx-auto mb-1" />
                        <p className="text-xl font-bold text-blue-600">{profile.rank?.level || 1}</p>
                        <p className="text-xs text-gray-500">当前等级</p>
                      </div>
                    </div>
                  </CardContent>
                </Card>

                {/* Achievements List */}
                <Card className="border-0 shadow-lg">
                  <CardContent className="p-6">
                    <div className="flex items-center justify-between mb-4">
                      <div className="flex items-center gap-3">
                        <div className="h-8 w-8 rounded-lg bg-amber-100 flex items-center justify-center">
                          <Award className="h-4 w-4 text-amber-600" />
                        </div>
                        <div>
                          <h2 className="text-base font-semibold text-gray-900">成就勋章</h2>
                          <p className="text-xs text-gray-500">
                            已解锁 {profile.completed_achievements}/{profile.total_achievements} 个成就
                          </p>
                        </div>
                      </div>
                    </div>

                    {achievements.length > 0 ? (
                      <div className="grid gap-3 sm:grid-cols-2">
                        {achievements.map((achievement) => (
                          <div
                            key={achievement.id}
                            className={cn(
                              "p-3 rounded-xl border-2 transition-all",
                              achievement.is_completed
                                ? "bg-gradient-to-br from-amber-50 to-orange-50 border-amber-200"
                                : "bg-gray-50 border-gray-200 opacity-60"
                            )}
                          >
                            <div className="flex items-start gap-3">
                              <div
                                className={cn(
                                  "h-10 w-10 rounded-xl flex items-center justify-center shrink-0",
                                  achievement.is_completed
                                    ? "bg-gradient-to-br from-amber-400 to-orange-500"
                                    : "bg-gray-300"
                                )}
                              >
                                {achievement.is_completed ? (
                                  <Star className="h-5 w-5 text-white" />
                                ) : (
                                  <Target className="h-5 w-5 text-gray-500" />
                                )}
                              </div>
                              <div className="flex-1 min-w-0">
                                <div className="flex items-center gap-2">
                                  <p className={cn(
                                    "font-semibold truncate text-sm",
                                    achievement.is_completed ? "text-gray-900" : "text-gray-500"
                                  )}>
                                    {achievement.name_zh}
                                  </p>
                                  {achievement.is_completed && (
                                    <span className="text-xs bg-green-100 text-green-600 px-1.5 py-0.5 rounded-full shrink-0">
                                      已获得
                                    </span>
                                  )}
                                </div>
                                <p className="text-xs text-gray-500 mt-0.5 line-clamp-1">
                                  {achievement.description_zh}
                                </p>
                                <div className="flex items-center gap-3 mt-1">
                                  <span className="text-xs text-amber-600 font-medium">
                                    +{achievement.xp_reward} XP
                                  </span>
                                  {!achievement.is_completed && achievement.requirement_value > 0 && (
                                    <div className="flex-1">
                                      <div className="h-1 bg-gray-200 rounded-full overflow-hidden">
                                        <div
                                          className="h-full bg-orange-400 rounded-full"
                                          style={{
                                            width: `${Math.min(100, (achievement.progress / achievement.requirement_value) * 100)}%`
                                          }}
                                        />
                                      </div>
                                    </div>
                                  )}
                                </div>
                              </div>
                            </div>
                          </div>
                        ))}
                      </div>
                    ) : (
                      <div className="text-center py-8 text-gray-500">
                        <Award className="h-10 w-10 mx-auto mb-3 text-gray-300" />
                        <p>暂无成就数据</p>
                      </div>
                    )}
                  </CardContent>
                </Card>
              </div>
            ) : (
              <div className="text-center py-12 text-gray-500">
                <Award className="h-12 w-12 mx-auto mb-4 text-gray-300" />
                <p>无法加载成就数据</p>
              </div>
            )}
          </TabsContent>

          {/* Settings Tab */}
          <TabsContent value="settings">
            <Card className="border-0 shadow-lg">
              <CardContent className="p-6">
                <div className="flex items-center gap-3 mb-6">
                  <div className="h-8 w-8 rounded-lg bg-purple-100 flex items-center justify-center">
                    <Bot className="h-4 w-4 text-purple-600" />
                  </div>
                  <div>
                    <h2 className="text-base font-semibold text-gray-900">AI 提供商设置</h2>
                    <p className="text-xs text-gray-500">配置您的 AI 服务提供商</p>
                  </div>
                </div>

                {aiProvidersLoading ? (
                  <div className="flex items-center justify-center py-8">
                    <Loader2 className="h-6 w-6 animate-spin text-purple-500" />
                  </div>
                ) : (
                  <div className="space-y-4">
                    {/* Provider Cards */}
                    <div className="grid gap-4 sm:grid-cols-2">
                      {/* Zhipu Provider */}
                      <ProviderCard
                        type="zhipu"
                        name="智谱 AI"
                        description="GLM 系列大模型"
                        provider={aiProviders.find(p => p.provider_type === 'zhipu')}
                        isSelected={selectedProviderType === 'zhipu'}
                        isTesting={providerTesting}
                        onSelect={() => {
                          setSelectedProviderType(selectedProviderType === 'zhipu' ? null : 'zhipu')
                          const existing = aiProviders.find(p => p.provider_type === 'zhipu')
                          setProviderFormData(existing?.settings as Record<string, string> || {})
                        }}
                        onTest={handleTestProvider}
                        onActivate={handleActivateProvider}
                        onDelete={handleDeleteProvider}
                      />

                      {/* Doubao Provider */}
                      <ProviderCard
                        type="doubao"
                        name="豆包"
                        description="字节跳动大模型"
                        provider={aiProviders.find(p => p.provider_type === 'doubao')}
                        isSelected={selectedProviderType === 'doubao'}
                        isTesting={providerTesting}
                        onSelect={() => {
                          setSelectedProviderType(selectedProviderType === 'doubao' ? null : 'doubao')
                          const existing = aiProviders.find(p => p.provider_type === 'doubao')
                          setProviderFormData(existing?.settings as Record<string, string> || {})
                        }}
                        onTest={handleTestProvider}
                        onActivate={handleActivateProvider}
                        onDelete={handleDeleteProvider}
                      />
                    </div>

                    {/* Configuration Form */}
                    {selectedProviderType && (
                      <Card className="border border-purple-200 bg-purple-50/50">
                        <CardContent className="p-4">
                          <h3 className="text-sm font-semibold text-gray-900 mb-4">
                            配置 {selectedProviderType === 'zhipu' ? '智谱 AI' : '豆包'}
                          </h3>

                          {selectedProviderType === 'zhipu' ? (
                            <div className="space-y-3">
                              <div>
                                <Label className="text-xs">API Key</Label>
                                <Input
                                  type="password"
                                  value={providerFormData.api_key || ''}
                                  onChange={(e) => setProviderFormData(prev => ({ ...prev, api_key: e.target.value }))}
                                  placeholder="输入 Zhipu API Key"
                                  className="h-9 mt-1"
                                />
                              </div>
                              <div>
                                <Label className="text-xs">Chat Model</Label>
                                <Input
                                  value={providerFormData.chat_model || 'glm-4-flash'}
                                  onChange={(e) => setProviderFormData(prev => ({ ...prev, chat_model: e.target.value }))}
                                  placeholder="glm-4-flash"
                                  className="h-9 mt-1"
                                />
                              </div>
                            </div>
                          ) : (
                            <div className="space-y-3">
                              <div>
                                <Label className="text-xs">App ID</Label>
                                <Input
                                  value={providerFormData.app_id || ''}
                                  onChange={(e) => setProviderFormData(prev => ({ ...prev, app_id: e.target.value }))}
                                  placeholder="输入 App ID"
                                  className="h-9 mt-1"
                                />
                              </div>
                              <div>
                                <Label className="text-xs">Access Token</Label>
                                <Input
                                  type="password"
                                  value={providerFormData.access_token || ''}
                                  onChange={(e) => setProviderFormData(prev => ({ ...prev, access_token: e.target.value }))}
                                  placeholder="输入 Access Token"
                                  className="h-9 mt-1"
                                />
                              </div>
                              <div>
                                <Label className="text-xs">API Key</Label>
                                <Input
                                  type="password"
                                  value={providerFormData.api_key || ''}
                                  onChange={(e) => setProviderFormData(prev => ({ ...prev, api_key: e.target.value }))}
                                  placeholder="输入 API Key"
                                  className="h-9 mt-1"
                                />
                              </div>
                              <div>
                                <Label className="text-xs">Chat Model</Label>
                                <Input
                                  value={providerFormData.chat_model || 'doubao-1-5-pro-32k-250115'}
                                  onChange={(e) => setProviderFormData(prev => ({ ...prev, chat_model: e.target.value }))}
                                  placeholder="doubao-1-5-pro-32k-250115"
                                  className="h-9 mt-1"
                                />
                              </div>
                            </div>
                          )}

                          <div className="flex gap-2 mt-4">
                            <Button
                              variant="outline"
                              size="sm"
                              onClick={() => {
                                setSelectedProviderType(null)
                                setProviderFormData({})
                              }}
                            >
                              取消
                            </Button>
                            <Button
                              size="sm"
                              onClick={handleSaveProvider}
                              disabled={providerSaving}
                              className="bg-purple-600 hover:bg-purple-700"
                            >
                              {providerSaving ? (
                                <>
                                  <Loader2 className="h-4 w-4 mr-1 animate-spin" />
                                  保存中...
                                </>
                              ) : (
                                '保存配置'
                              )}
                            </Button>
                          </div>
                        </CardContent>
                      </Card>
                    )}
                  </div>
                )}
              </CardContent>
            </Card>
          </TabsContent>
        </Tabs>
      </main>

      <Footer />

      {/* Avatar Crop Dialog */}
      {cropDialogImage && (
        <AvatarCropDialog
          imageUrl={cropDialogImage}
          onCrop={handleCroppedImage}
          onCancel={handleCropCancel}
        />
      )}

      {/* Password Change Dialog */}
      <Dialog open={passwordDialogOpen} onOpenChange={(open) => {
        setPasswordDialogOpen(open)
        if (!open) {
          setCurrentPassword('')
          setNewPassword('')
          setConfirmPassword('')
          setPasswordError(null)
        }
      }}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>{accountStatus?.has_password ? '修改密码' : '设置密码'}</DialogTitle>
            <DialogDescription>
              {accountStatus?.has_password
                ? '请输入当前密码和新密码'
                : '设置密码后可以使用邮箱和密码登录'}
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-2">
            {accountStatus?.has_password && (
              <div className="space-y-2">
                <Label htmlFor="current-password">当前密码</Label>
                <Input
                  id="current-password"
                  type="password"
                  value={currentPassword}
                  onChange={(e) => setCurrentPassword(e.target.value)}
                  placeholder="输入当前密码"
                />
              </div>
            )}
            <div className="space-y-2">
              <Label htmlFor="new-password">新密码</Label>
              <Input
                id="new-password"
                type="password"
                value={newPassword}
                onChange={(e) => setNewPassword(e.target.value)}
                placeholder="至少 8 个字符"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="confirm-password">确认新密码</Label>
              <Input
                id="confirm-password"
                type="password"
                value={confirmPassword}
                onChange={(e) => setConfirmPassword(e.target.value)}
                placeholder="再次输入新密码"
              />
            </div>
            {passwordError && (
              <p className="text-sm text-red-500">{passwordError}</p>
            )}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setPasswordDialogOpen(false)}>
              取消
            </Button>
            <Button
              onClick={handleChangePassword}
              disabled={passwordSaving}
              className="bg-gradient-to-r from-orange-500 to-amber-500 hover:from-orange-600 hover:to-amber-600"
            >
              {passwordSaving ? (
                <>
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                  保存中...
                </>
              ) : (
                '确认'
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}

// Provider Card Component
function ProviderCard({
  type: _type,
  name,
  description,
  provider,
  isSelected,
  isTesting,
  onSelect,
  onTest,
  onActivate,
  onDelete,
}: {
  type: AiProviderType
  name: string
  description: string
  provider?: UserAiProvider
  isSelected: boolean
  isTesting: number | null
  onSelect: () => void
  onTest: (id: number) => void
  onActivate: (id: number) => void
  onDelete: (id: number) => void
}) {
  const isConfigured = !!provider
  const isActive = provider?.is_active
  const testStatus = provider?.test_status

  return (
    <div
      className={cn(
        "p-4 rounded-xl border-2 transition-all cursor-pointer",
        isSelected
          ? "border-purple-400 bg-purple-50"
          : isActive
            ? "border-green-300 bg-green-50/50"
            : isConfigured
              ? "border-gray-200 bg-white hover:border-purple-200"
              : "border-dashed border-gray-300 bg-gray-50 hover:border-purple-200"
      )}
      onClick={onSelect}
    >
      <div className="flex items-start justify-between">
        <div className="flex items-center gap-3">
          <div className={cn(
            "h-10 w-10 rounded-lg flex items-center justify-center",
            isActive ? "bg-green-100" : isConfigured ? "bg-purple-100" : "bg-gray-200"
          )}>
            <Bot className={cn(
              "h-5 w-5",
              isActive ? "text-green-600" : isConfigured ? "text-purple-600" : "text-gray-400"
            )} />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <p className="font-semibold text-gray-900">{name}</p>
              {isActive && (
                <span className="text-xs bg-green-100 text-green-600 px-1.5 py-0.5 rounded-full">
                  使用中
                </span>
              )}
            </div>
            <p className="text-xs text-gray-500">{description}</p>
          </div>
        </div>

        {testStatus && (
          <div className="shrink-0">
            {testStatus === 'success' ? (
              <CheckCircle className="h-5 w-5 text-green-500" />
            ) : (
              <XCircle className="h-5 w-5 text-red-500" />
            )}
          </div>
        )}
      </div>

      {/* Error message display */}
      {provider?.test_error && testStatus === 'failed' && (
        <div className="mt-2 p-2 bg-red-50 border border-red-200 rounded-lg">
          <p className="text-xs text-red-600 break-words">{provider.test_error}</p>
        </div>
      )}

      {isConfigured && (
        <div className="flex gap-2 mt-3" onClick={(e) => e.stopPropagation()}>
          <Button
            variant="outline"
            size="sm"
            className="h-7 text-xs"
            onClick={() => onTest(provider.id)}
            disabled={isTesting === provider.id}
          >
            {isTesting === provider.id ? (
              <Loader2 className="h-3 w-3 animate-spin" />
            ) : (
              <>
                <Play className="h-3 w-3 mr-1" />
                测试
              </>
            )}
          </Button>
          {!isActive && (
            <Button
              variant="outline"
              size="sm"
              className="h-7 text-xs text-green-600 hover:text-green-700"
              onClick={() => onActivate(provider.id)}
            >
              激活
            </Button>
          )}
          <Button
            variant="outline"
            size="sm"
            className="h-7 text-xs text-red-600 hover:text-red-700"
            onClick={() => onDelete(provider.id)}
          >
            删除
          </Button>
        </div>
      )}
    </div>
  )
}
