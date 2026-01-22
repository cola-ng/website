import { useEffect, useState, useRef } from 'react'
import { Navigate } from 'react-router-dom'
import { Camera, Mail, Phone, Calendar, Shield, User, Loader2 } from 'lucide-react'

import { Card, CardContent } from '../components/ui/card'
import { Button } from '../components/ui/button'
import { Input } from '../components/ui/input'
import { Label } from '../components/ui/label'
import { Footer } from '../components/Footer'
import { Header } from '../components/Header'
import { me, updateMe, uploadAvatar, deleteAvatar, getAvatarUrl } from '../lib/api'
import { useAuth } from '../lib/auth'

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
  const fileInputRef = useRef<HTMLInputElement>(null)

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

  const handleAvatarChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (!file || !token) return

    // Validate file type
    if (!file.type.startsWith('image/')) {
      setAvatarError('请选择图片文件')
      return
    }

    // Validate file size (max 5MB)
    if (file.size > 5 * 1024 * 1024) {
      setAvatarError('图片大小不能超过 5MB')
      return
    }

    setAvatarUploading(true)
    setAvatarError(null)

    try {
      const updatedUser = await uploadAvatar(token, file)
      setAuth(token, updatedUser)
    } catch (e: unknown) {
      setAvatarError(e instanceof Error ? e.message : '上传失败')
    } finally {
      setAvatarUploading(false)
      // Reset file input
      if (fileInputRef.current) {
        fileInputRef.current.value = ''
      }
    }
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

  if (!token) {
    return <Navigate to="/" replace />
  }

  const avatarUrl = getAvatarUrl(user, 320)

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-50 via-gray-50 to-zinc-100">
      <Header />

      <main className="mx-auto max-w-4xl px-4 py-8">
        {/* Profile Header Card */}
        <Card className="mb-6 overflow-hidden border-0 shadow-xl">
          {/* Banner */}
          <div className="h-32 bg-gradient-to-r from-orange-500 via-amber-500 to-orange-400" />

          {/* Avatar & Name Section */}
          <CardContent className="relative pb-6">
            <div className="flex flex-col sm:flex-row sm:items-end gap-4 -mt-16">
              {/* Avatar */}
              <div className="relative group">
                <div className="h-32 w-32 rounded-2xl bg-white p-1 shadow-xl">
                  <div className="h-full w-full rounded-xl bg-gradient-to-br from-orange-500 to-amber-400 flex items-center justify-center overflow-hidden">
                    {avatarUrl ? (
                      <img
                        src={avatarUrl}
                        alt="Avatar"
                        className="h-full w-full object-cover"
                      />
                    ) : (
                      <User className="h-16 w-16 text-white" />
                    )}
                  </div>
                </div>

                {/* Avatar Upload Overlay */}
                <button
                  onClick={handleAvatarClick}
                  disabled={avatarUploading}
                  className="absolute inset-1 rounded-xl bg-black/50 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center cursor-pointer disabled:cursor-not-allowed"
                >
                  {avatarUploading ? (
                    <Loader2 className="h-8 w-8 text-white animate-spin" />
                  ) : (
                    <Camera className="h-8 w-8 text-white" />
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
              <div className="flex-1 pt-4 sm:pt-0 sm:pb-2">
                <h1 className="text-2xl font-bold text-gray-900">
                  {user?.name || user?.email?.split('@')[0] || '用户'}
                </h1>
                <p className="text-gray-500 mt-1">{user?.email}</p>
                {avatarError && (
                  <p className="text-sm text-red-500 mt-2">{avatarError}</p>
                )}
              </div>

              {/* Action Buttons */}
              <div className="flex gap-2 sm:pb-2">
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
            </div>
          </CardContent>
        </Card>

        <div className="grid gap-6 md:grid-cols-2">
          {/* Account Information */}
          <Card className="border-0 shadow-lg">
            <CardContent className="p-6">
              <div className="flex items-center gap-3 mb-6">
                <div className="h-10 w-10 rounded-lg bg-blue-100 flex items-center justify-center">
                  <Shield className="h-5 w-5 text-blue-600" />
                </div>
                <div>
                  <h2 className="text-lg font-semibold text-gray-900">账户信息</h2>
                  <p className="text-sm text-gray-500">您的账户详情</p>
                </div>
              </div>

              <div className="space-y-4">
                <div className="flex items-center gap-3 p-3 rounded-lg bg-gray-50">
                  <Mail className="h-5 w-5 text-gray-400" />
                  <div className="flex-1 min-w-0">
                    <p className="text-xs text-gray-500">邮箱地址</p>
                    <p className="text-sm font-medium text-gray-900 truncate">{user?.email || '-'}</p>
                  </div>
                </div>

                <div className="flex items-center gap-3 p-3 rounded-lg bg-gray-50">
                  <Phone className="h-5 w-5 text-gray-400" />
                  <div className="flex-1 min-w-0">
                    <p className="text-xs text-gray-500">手机号码</p>
                    <p className="text-sm font-medium text-gray-900">{user?.phone || '未设置'}</p>
                  </div>
                </div>

                <div className="flex items-center gap-3 p-3 rounded-lg bg-gray-50">
                  <Calendar className="h-5 w-5 text-gray-400" />
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
              <div className="flex items-center gap-3 mb-6">
                <div className="h-10 w-10 rounded-lg bg-orange-100 flex items-center justify-center">
                  <User className="h-5 w-5 text-orange-600" />
                </div>
                <div>
                  <h2 className="text-lg font-semibold text-gray-900">编辑资料</h2>
                  <p className="text-sm text-gray-500">更新您的个人信息</p>
                </div>
              </div>

              <div className="space-y-4">
                <div className="space-y-2">
                  <Label htmlFor="edit-name" className="text-sm font-medium">
                    显示名称
                  </Label>
                  <Input
                    id="edit-name"
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                    placeholder="输入您的姓名"
                    className="h-10"
                  />
                </div>

                <div className="space-y-2">
                  <Label htmlFor="edit-phone" className="text-sm font-medium">
                    手机号码
                  </Label>
                  <Input
                    id="edit-phone"
                    value={phone}
                    onChange={(e) => setPhone(e.target.value)}
                    placeholder="输入您的手机号码"
                    className="h-10"
                  />
                </div>

                <div className="pt-2">
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

                  {saved && (
                    <p className="text-sm text-green-600 text-center mt-3">
                      资料更新成功！
                    </p>
                  )}
                  {saveError && (
                    <p className="text-sm text-red-500 text-center mt-3">{saveError}</p>
                  )}
                </div>
              </div>
            </CardContent>
          </Card>
        </div>
      </main>

      <Footer />
    </div>
  )
}
