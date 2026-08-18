import { SHOWCASE_COMPONENT_DEFS } from "./registry";
import { SHOWCASE_GRID_COLUMNS } from "./types";
import type { ShowcaseContext, ShowcaseLayout } from "./types";

// Tailwind 需要静态 class 字符串才能被 JIT 收集；按列数映射到显式类名
const COL_SPAN_CLASS: Record<number, string> = {
  1: "lg:col-span-1",
  2: "lg:col-span-2",
  3: "lg:col-span-3",
  4: "lg:col-span-4",
};

/**
 * 展板栅格渲染层。
 * 按组件 position.order 排序后，以 4 列栅格（lg 及以上；小屏单列）排布：
 *   - 宽度 = size.width（1..4 列）
 *   - 最小高度 = size.height（px）
 *   - 顺序 = position.order
 * 未知类型组件（未来扩展）跳过渲染，但保留在布局数据中。
 */
export default function ShowcaseBoard({
  layout,
  ctx,
}: {
  layout: ShowcaseLayout;
  ctx: ShowcaseContext;
}) {
  const ordered = [...layout.components]
    .sort((a, b) => a.position.order - b.position.order)
    .filter((c) => c.enabled);

  if (ordered.length === 0) {
    return (
      <div className="py-12 text-center text-gray-400 dark:text-gray-500">
        展板暂无内容
      </div>
    );
  }

  return (
    <div className="grid grid-cols-1 lg:grid-cols-4 gap-6 items-start">
      {ordered.map((config) => {
        const def = SHOWCASE_COMPONENT_DEFS[config.type];
        if (!def) return null;
        const width = Math.min(
          SHOWCASE_GRID_COLUMNS,
          Math.max(1, Math.floor(config.size?.width ?? SHOWCASE_GRID_COLUMNS)),
        );
        const height =
          typeof config.size?.height === "number" && config.size.height > 0
            ? config.size.height
            : undefined;
        const Component = def.component;
        return (
          <div
            key={config.id}
            className={`${COL_SPAN_CLASS[width] ?? COL_SPAN_CLASS[SHOWCASE_GRID_COLUMNS]} min-w-0`}
            style={height ? { minHeight: height } : undefined}
          >
            <Component config={config} ctx={ctx} />
          </div>
        );
      })}
    </div>
  );
}