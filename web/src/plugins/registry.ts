import React, { lazy } from 'react'
import type { ComponentType, LazyExoticComponent } from 'react'
import { apiFetch } from '../services/api'
import type { PluginManifest, PluginRouteDef, PluginApiResponse } from './types'

export interface PluginRegistration {
  manifest: PluginManifest
  routes: PluginRouteDef[]
  component: LazyExoticComponent<ComponentType<unknown>>
}

interface PluginsListResponse {
  plugins: PluginManifest[]
}

type RegistryListener = () => void

class PluginRegistry {
  private static instance: PluginRegistry
  private plugins = new Map<string, PluginRegistration>()
  private listeners = new Set<RegistryListener>()
  private mainNavItems: PluginRouteDef[] = []
  private adminNavItems: PluginRouteDef[] = []
  private pluginRoutes: Array<{ pluginId: string; route: PluginRouteDef }> = []

  static getInstance(): PluginRegistry {
    if (!PluginRegistry.instance) {
      PluginRegistry.instance = new PluginRegistry()
    }
    return PluginRegistry.instance
  }

  subscribe(listener: RegistryListener): () => void {
    this.listeners.add(listener)
    return () => {
      this.listeners.delete(listener)
    }
  }

  private notify(): void {
    this.rebuildCaches()
    for (const listener of this.listeners) {
      listener()
    }
  }

  private rebuildCaches(): void {
    this.mainNavItems = []
    this.adminNavItems = []
    this.pluginRoutes = []
    for (const [pluginId, reg] of this.plugins.entries()) {
      for (const route of reg.routes) {
        this.pluginRoutes.push({ pluginId, route })
        if (route.nav_placement === 'main') {
          this.mainNavItems.push(route)
        } else if (route.nav_placement === 'admin') {
          this.adminNavItems.push(route)
        }
      }
    }
  }

  async discover(): Promise<void> {
    try {
      const [manifestsRes, routesRes] = await Promise.all([
        apiFetch<PluginsListResponse>('/api/plugins'),
        apiFetch<PluginApiResponse>('/api/plugins/routes'),
      ])

      const manifests = new Map<string, PluginManifest>()
      for (const manifest of manifestsRes.plugins) {
        manifests.set(manifest.id, manifest)
      }

      for (const plugin of routesRes.plugins) {
        const manifest = manifests.get(plugin.id) ?? plugin.manifest
        const routes = plugin.routes

        const PlaceholderComponent = lazy<ComponentType<unknown>>(() =>
          import('./placeholder').catch(() => ({
            default: (() =>
              React.createElement(
                'div',
                { className: 'p-6 text-center text-gray-500 dark:text-gray-400' },
                `插件 "${manifest?.name ?? plugin.id}" 加载中...`,
              )
            ) as ComponentType<unknown>,
          })),
        )

        this.plugins.set(plugin.id, {
          manifest,
          routes,
          component: PlaceholderComponent,
        })
      }
      this.notify()
    } catch (err) {
      console.warn('[PluginRegistry] Failed to discover plugins:', err)
    }
  }

  getMainNavItems(): PluginRouteDef[] {
    return this.mainNavItems
  }

  getAdminNavItems(): PluginRouteDef[] {
    return this.adminNavItems
  }

  getPluginRoutes(): Array<{ pluginId: string; route: PluginRouteDef }> {
    return this.pluginRoutes
  }

  getComponent(pluginId: string): LazyExoticComponent<ComponentType<unknown>> | null {
    return this.plugins.get(pluginId)?.component ?? null
  }

  isLoaded(pluginId: string): boolean {
    return this.plugins.has(pluginId)
  }

  registerComponent(
    pluginId: string,
    component: LazyExoticComponent<ComponentType<unknown>>,
  ): void {
    const reg = this.plugins.get(pluginId)
    if (reg) {
      reg.component = component
    }
  }
}

export { PluginRegistry }
