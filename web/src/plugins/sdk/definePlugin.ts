import { lazy, type ComponentType, type LazyExoticComponent } from 'react'
import { PluginRegistry } from '../registry'
import type { PluginDefinition } from '../types'

/**
 * Declare a React plugin and register it with the PluginRegistry.
 *
 * Usage:
 *   // my-plugin/index.ts
 *   import { definePlugin } from '../sdk/definePlugin'
 *   import MyPage from './MyPage'
 *
 *   const plugin = definePlugin({
 *     id: 'my-plugin',
 *     name: 'My Plugin',
 *     version: '1.0.0',
 *     routes: [
 *       { path: '/plugins/my', label: 'My Plugin', icon: '🔌', nav_placement: 'main' },
 *     ],
 *     slots: [
 *       { slot: 'member_card_actions', component: MyButton },
 *     ],
 *   }, lazy(() => import('./MyPage')))
 *
 *   export default plugin
 */
export function definePlugin(
  definition: PluginDefinition,
  component?: LazyExoticComponent<ComponentType<unknown>>,
): PluginDefinition {
  const registry = PluginRegistry.getInstance()
  registry.register(definition, component)
  return definition
}
