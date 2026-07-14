import { Suspense } from 'react'
import { useParams } from 'react-router-dom'
import { PluginRegistry } from './registry'
import LoadingSpinner from '../components/ui/LoadingSpinner'

interface PluginPageProps {
  pluginId?: string
}

export default function PluginPage({ pluginId: pluginIdProp }: PluginPageProps) {
  const { pluginId: pluginIdParam } = useParams<{ pluginId: string }>()
  const pluginId = pluginIdProp ?? pluginIdParam
  const component = pluginId ? PluginRegistry.getInstance().getComponent(pluginId) : null

  if (!component) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <p className="text-gray-500 dark:text-gray-400">插件未找到或未加载</p>
      </div>
    )
  }

  const Component = component
  return (
    <Suspense fallback={<LoadingSpinner />}>
      <Component />
    </Suspense>
  )
}
