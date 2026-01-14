import * as React from 'react'

import { Button } from './ui/button'
import { OAUTH_PROVIDERS, type OAuthProviderKey } from '../lib/oauth-config'
import { Github, Chrome } from 'lucide-react'

interface OAuthButtonProps {
  provider: OAuthProviderKey
  onLoginStart: (provider: string, userId: string, email?: string) => void
}

export function OAuthButton({ provider, onLoginStart }: OAuthButtonProps) {
  const config = OAUTH_PROVIDERS[provider]

  if (!config?.enabled) {
    return null
  }

  const handleClick = () => {
    // In a real implementation, this would redirect to the OAuth provider's auth endpoint
    // For demo purposes, we'll show a prompt
    const userId = prompt(`Enter your ${config.name} user ID:`)
    const email = prompt(`Enter your ${config.name} email (optional):`)

    if (userId) {
      onLoginStart(provider, userId, email || undefined)
    }
  }

  return (
    <Button
      variant="outline"
      className="w-full"
      onClick={handleClick}
    >
      {provider === 'google' ? (
        <>
          <Chrome className="mr-2 h-4 w-4" />
          Sign in with {config.name}
        </>
      ) : (
        <>
          <Github className="mr-2 h-4 w-4" />
          Sign in with {config.name}
        </>
      )}
    </Button>
  )
}
