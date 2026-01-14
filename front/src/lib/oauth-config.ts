export const OAUTH_PROVIDERS = {
  google: {
    name: 'Google',
    enabled: true,
  },
  github: {
    name: 'GitHub',
    enabled: true,
  },
} as const

export type OAuthProviderKey = keyof typeof OAUTH_PROVIDERS

export interface OAuthProvider {
  name: string
  enabled: boolean
}
