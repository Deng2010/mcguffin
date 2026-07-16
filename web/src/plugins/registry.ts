import { lazy } from 'react'
import type { ComponentType, LazyExoticComponent } from 'react'
import { apiFetch } from '../services/api'
import type { PluginManifest, PluginRouteDef, PluginDefinition, PluginSlotDef } from './types'

export interface PluginRegistration {
  manifest: PluginManifest
  routes: PluginRouteDef[]
  component: LazyExoticComponent<ComponentType<unknown>>
  slots: PluginSlotDef[]
}

type RegistryListener = () => void

class PluginRegistry {
  private static instance: PluginRegistry
  private plugins = new Map<string, PluginRegistration>()
  private listeners = new Set<RegistryListener>()
  private mainNavItems: PluginRouteDef[] = []
  private adminNavItems: PluginRouteDef[] = []
  private pluginRoutes: Array<{ pluginId: string; route: PluginRouteDef }> = []
  /** slot name → components */
  private slotComponents = new Map<string, Array<{ pluginId: string; component: ComponentType<any> }>>()
  private discovered = false

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

  /** Register a plugin definition from a `definePlugin()` call. */
  register(definition: PluginDefinition, component?: LazyExoticComponent<ComponentType<unknown>>): void {
    const manifest: PluginManifest = {
      id: definition.id,
      name: definition.name,
      version: definition.version,
      description: definition.description ?? '',
      author: definition.author,
      permissions_needed: definition.permissions_needed ?? [],
    }

    const routes = definition.routes ?? []
    const slots = definition.slots ?? []

    // Store slots
    for (const slot of slots) {
      if (!this.slotComponents.has(slot.slot)) {
        this.slotComponents.set(slot.slot, [])
      }
      this.slotComponents.get(slot.slot)!.push({ pluginId: definition.id, component: slot.component })
    }

    const pageComponent = component ?? lazy<ComponentType<unknown>>(() =>
      import('./PluginPage').then(m => ({ default: m.default as ComponentType<unknown> }))
    )

    this.plugins.set(definition.id, {
      manifest,
      routes,
      component: pageComponent,
      slots,
    })

    this.notify()

    // Tell the backend about this plugin (for data API authorization)
    this.syncToBackend(definition).catch(() => {})
  }

  /** Sync plugin metadata to backend so data APIs recognize it. */
  private async syncToBackend(definition: PluginDefinition): Promise<void> {
    try {
      await apiFetch('/plugins/register', {
        method: 'POST',
        body: JSON.stringify({
          id: definition.id,
          manifest: {
            id: definition.id,
            name: definition.name,
            version: definition.version,
            description: definition.description ?? '',
            permissions_needed: definition.permissions_needed ?? [],
          },
          routes: definition.routes ?? [],
          permissions: [],
        }),
      })
    } catch {
      // Backend may not be available during dev; ignore
    }
  }

  /** Get slot components for a named slot. */
  getSlotComponents(slot: string): Array<{ pluginId: string; component: ComponentType<any> }> {
    return this.slotComponents.get(slot) ?? []
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

  /**
   * Auto-discover plugins by dynamically importing modules under `web/src/plugins/`.
   *
   * Uses Vite's `import.meta.glob` to find plugin entry points. Each discovered
   * module that calls `definePlugin()` will self-register via the `register()` method.
   *
   * Convention: files matching `**\/*.plugin.ts` or `**\/index.ts` under
   * `src/plugins/` are treated as potential plugin entry points.
   */
  discover(): void {
    if (this.discovered) return
    this.discovered = true

    // Use Vite's import.meta.glob to discover plugin entry points.
    // The glob pattern is resolved at build time.
    const pluginModules = import.meta.glob('/src/plugins/**/*.plugin.ts')

    // Also scan for index.ts in plugin subdirectories
    const pluginIndexModules = import.meta.glob('/src/plugins/*/index.ts')

    const allModules = { ...pluginModules, ...pluginIndexModules }
    const paths = Object.keys(allModules)

    if (paths.length === 0) {
      // No plugin files found — this is fine; plugins register manually too.
      tracing.info?.('[plugin] no plugin entry points found via glob')
    }

    // Dynamically import all discovered modules.
    // Each module that calls definePlugin() will self-register.
    for (const [path, importer] of Object.entries(allModules)) {
      importer().catch((err: unknown) => {
        console.error(`[plugin] failed to load ${path}:`, err)
      })
    }
  }

  isLoaded(pluginId: string): boolean {
    return this.plugins.has(pluginId)
  }
}

// Minimal tracing-like shim for the registry itself
const tracing = {
  info: (msg: string) => {
    if (import.meta.env.DEV) {
      // eslint-disable-next-line no-console
      console.log(msg)
    }
  },
}

export { PluginRegistry }
