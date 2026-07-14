export interface PluginManifest {
  id: string
  name: string
  version: string
  description: string
  author?: string
  homepage?: string
  permissions_needed: string[]
}

export interface PluginRouteDef {
  path: string
  label: string
  icon?: string
  required_permission?: string
  nav_placement: 'main' | 'admin' | 'hidden'
}

export interface PluginApiResponse {
  plugins: Array<{
    id: string
    manifest: PluginManifest
    routes: PluginRouteDef[]
  }>
}
