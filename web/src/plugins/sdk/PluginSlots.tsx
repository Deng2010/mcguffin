import { PluginRegistry } from '../registry'

interface PluginSlotsProps {
  slot: string
  props?: Record<string, unknown>
}

/**
 * Render all plugin components registered for a named slot.
 * Used like:
 *   <PluginSlots slot="member_card_actions" props={{ member }} />
 */
export default function PluginSlots({ slot, props = {} }: PluginSlotsProps) {
  const registry = PluginRegistry.getInstance()
  const components = registry.getSlotComponents(slot)

  if (components.length === 0) return null

  return (
    <>
      {components.map(({ pluginId, component: Component }) => (
        <Component key={pluginId} {...props} />
      ))}
    </>
  )
}
