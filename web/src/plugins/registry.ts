import { lazy } from "react";
import type { ComponentType, LazyExoticComponent } from "react";
import { apiFetch } from "../services/api";
import type {
  PluginManifest,
  PluginRouteDef,
  PluginDefinition,
  PluginSlotDef,
} from "./types";

export interface PluginRegistration {
  manifest: PluginManifest;
  routes: PluginRouteDef[];
  component: LazyExoticComponent<ComponentType<unknown>>;
  slots: PluginSlotDef[];
}

type RegistryListener = () => void;

class PluginRegistry {
  private static instance: PluginRegistry;
  private plugins = new Map<string, PluginRegistration>();
  private listeners = new Set<RegistryListener>();
  private mainNavItems: PluginRouteDef[] = [];
  private adminNavItems: PluginRouteDef[] = [];
  private pluginRoutes: Array<{ pluginId: string; route: PluginRouteDef }> = [];
  /** slot name → components */
  private slotComponents = new Map<
    string,
    Array<{ pluginId: string; component: ComponentType<any> }>
  >();
  /** IDs of plugins that are disabled on the backend */
  private disabledPluginIds = new Set<string>();
  private statusFetched = false;
  /** Whether the plugin feature is globally disabled by an admin */
  private globallyDisabled = false;
  /** pluginId → 远程（zip）插件加载失败原因，用于管理页展示 */
  private loadErrors = new Map<string, string>();

  static getInstance(): PluginRegistry {
    if (!PluginRegistry.instance) {
      PluginRegistry.instance = new PluginRegistry();
    }
    return PluginRegistry.instance;
  }

  subscribe(listener: RegistryListener): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  private notify(): void {
    this.rebuildCaches();
    for (const listener of this.listeners) {
      listener();
    }
  }

  private rebuildCaches(): void {
    this.mainNavItems = [];
    this.adminNavItems = [];
    this.pluginRoutes = [];
    // When the plugin feature is globally disabled, hide everything.
    if (this.globallyDisabled) return;
    for (const [pluginId, reg] of this.plugins.entries()) {
      if (this.disabledPluginIds.has(pluginId)) continue;
      for (const route of reg.routes) {
        this.pluginRoutes.push({ pluginId, route });
        if (route.nav_placement === "main") {
          this.mainNavItems.push(route);
        } else if (route.nav_placement === "admin") {
          this.adminNavItems.push(route);
        }
      }
    }
  }

  /**
   * Register a plugin definition. Called through `definePlugin()` by the entry
   * module of a ZIP-installed plugin when it is dynamically imported.
   */
  register(
    definition: PluginDefinition,
    component?: LazyExoticComponent<ComponentType<unknown>>,
  ): void {
    const manifest: PluginManifest = {
      id: definition.id,
      name: definition.name,
      version: definition.version,
      description: definition.description ?? "",
      author: definition.author,
      permissions_needed: definition.permissions_needed ?? [],
      enabled: true,
    };

    const routes = definition.routes ?? [];
    const slots = definition.slots ?? [];

    // 去重：重复注册（HMR / 页面重载）时先清掉该插件旧的插槽组件，
    // 避免同一插槽组件被渲染多次。
    for (const [slotName, list] of this.slotComponents.entries()) {
      const filtered = list.filter((s) => s.pluginId !== definition.id);
      if (filtered.length !== list.length) {
        this.slotComponents.set(slotName, filtered);
      }
    }

    // Store slots
    for (const slot of slots) {
      if (!this.slotComponents.has(slot.slot)) {
        this.slotComponents.set(slot.slot, []);
      }
      this.slotComponents
        .get(slot.slot)!
        .push({ pluginId: definition.id, component: slot.component });
    }

    const pageComponent =
      component ??
      lazy<ComponentType<unknown>>(() =>
        import("./PluginPage").then((m) => ({
          default: m.default as ComponentType<unknown>,
        })),
      );

    this.plugins.set(definition.id, {
      manifest,
      routes,
      component: pageComponent,
      slots,
    });

    this.notify();

    // Tell the backend about this plugin (for data API authorization)
    this.syncToBackend(definition).catch(() => {});
  }

  /** Sync plugin metadata to backend so data APIs recognize it. */
  private async syncToBackend(definition: PluginDefinition): Promise<void> {
    try {
      await apiFetch("/plugins/register", {
        method: "POST",
        body: JSON.stringify({
          id: definition.id,
          manifest: {
            id: definition.id,
            name: definition.name,
            version: definition.version,
            description: definition.description ?? "",
            permissions_needed: definition.permissions_needed ?? [],
          },
          routes: definition.routes ?? [],
          permissions: definition.permissions_needed ?? [],
        }),
      });
    } catch {
      // Backend may not be available during dev; ignore
    }
  }

  /** Fetch plugin enabled status from the backend. Call once on app init. */
  async fetchPluginStatus(): Promise<void> {
    if (this.statusFetched) return;
    this.statusFetched = true;
    await this.refreshRemotePlugins();
  }

  /**
   * Re-fetch plugin status from the backend and dynamically load any
   * zip-installed plugins that are not loaded yet. Safe to call multiple
   * times (e.g. after installing / enabling a plugin in the admin page).
   */
  async refreshRemotePlugins(): Promise<void> {
    try {
      const res = await apiFetch<{
        plugins: Array<{
          id: string;
          enabled: boolean;
          source?: "code" | "zip";
          entry?: string;
        }>;
        plugins_disabled?: boolean;
      }>("/plugins");
      const disabled = new Set<string>();
      for (const p of res.plugins) {
        if (!p.enabled) {
          disabled.add(p.id);
        }
      }
      this.disabledPluginIds = disabled;
      this.globallyDisabled = res.plugins_disabled === true;
      this.notify();
      await this.loadRemotePlugins(res.plugins);
    } catch {
      // Backend may not be available; treat all as enabled
    }
  }

  /**
   * Dynamically import entry modules of zip-installed plugins.
   * Each entry is an ESM that registers itself via
   * `window.__MCGUFFIN_SDK__.definePlugin()`.
   */
  private async loadRemotePlugins(
    plugins: Array<{
      id: string;
      enabled: boolean;
      source?: "code" | "zip";
      entry?: string;
    }>,
  ): Promise<void> {
    if (this.globallyDisabled) return;
    for (const p of plugins) {
      if (p.source !== "zip" || !p.enabled) continue;
      if (this.isLoaded(p.id)) continue;
      const entry = p.entry ?? "index.js";
      const url = `/api/plugins/${encodeURIComponent(p.id)}/assets/${entry
        .split("/")
        .map(encodeURIComponent)
        .join("/")}`;
      try {
        await import(/* @vite-ignore */ url);
        this.loadErrors.delete(p.id);
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        this.loadErrors.set(p.id, msg);
        console.error("[plugin] failed to load remote plugin %s:", p.id, err);
      }
    }
  }

  /** Get the load error of a remote plugin, if any. */
  getPluginLoadError(pluginId: string): string | undefined {
    return this.loadErrors.get(pluginId);
  }

  /**
   * Remove a plugin from the local registry (routes, nav items, slots).
   * Called after uninstalling a zip plugin; the already-imported ESM module
   * itself cannot be unloaded, but it becomes unreachable.
   */
  remove(pluginId: string): void {
    const existed = this.plugins.delete(pluginId);
    for (const [slotName, list] of this.slotComponents.entries()) {
      const filtered = list.filter((s) => s.pluginId !== pluginId);
      if (filtered.length !== list.length) {
        this.slotComponents.set(slotName, filtered);
      }
    }
    this.loadErrors.delete(pluginId);
    if (existed) this.notify();
  }

  /** Check whether a plugin is currently enabled. */
  isPluginEnabled(pluginId: string): boolean {
    return !this.globallyDisabled && !this.disabledPluginIds.has(pluginId);
  }

  /** Whether the plugin feature is globally disabled by an admin. */
  isGloballyDisabled(): boolean {
    return this.globallyDisabled;
  }

  /**
   * Update the local global-disabled state (called after an admin toggles it,
   * so the nav/routes re-render immediately without a full page reload).
   */
  setGloballyDisabled(disabled: boolean): void {
    if (this.globallyDisabled === disabled) return;
    this.globallyDisabled = disabled;
    this.notify();
  }

  /** Get slot components for a named slot. Disabled plugins are excluded. */
  getSlotComponents(
    slot: string,
  ): Array<{ pluginId: string; component: ComponentType<any> }> {
    const all = this.slotComponents.get(slot) ?? [];
    if (this.globallyDisabled) return [];
    return all.filter((s) => !this.disabledPluginIds.has(s.pluginId));
  }

  getMainNavItems(): PluginRouteDef[] {
    return this.mainNavItems;
  }

  getAdminNavItems(): PluginRouteDef[] {
    return this.adminNavItems;
  }

  getPluginRoutes(): Array<{ pluginId: string; route: PluginRouteDef }> {
    return this.pluginRoutes;
  }

  getComponent(
    pluginId: string,
  ): LazyExoticComponent<ComponentType<unknown>> | null {
    return this.plugins.get(pluginId)?.component ?? null;
  }

  isLoaded(pluginId: string): boolean {
    return this.plugins.has(pluginId);
  }
}

export { PluginRegistry };
