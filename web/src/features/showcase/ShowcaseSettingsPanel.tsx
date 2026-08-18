import { SHOWCASE_COMPONENT_DEFS, type SettingField } from "./registry";
import { SHOWCASE_GRID_COLUMNS } from "./types";
import type {
  ShowcaseComponentConfig,
  ShowcaseContext,
  ShowcaseLayout,
} from "./types";

interface ShowcaseSettingsPanelProps {
  layout: ShowcaseLayout;
  onChange: (layout: ShowcaseLayout) => void;
  ctx: ShowcaseContext;
  saving: boolean;
  msg: string;
  onSave: () => void;
  onCancel: () => void;
}

const WIDTH_LABELS: Record<number, string> = {
  1: "1/4 行",
  2: "1/2 行",
  3: "3/4 行",
  4: "整行",
};

/** 在 components 数组上应用更新并保持 order 与下标一致 */
function reindex(components: ShowcaseComponentConfig[]): ShowcaseComponentConfig[] {
  return components.map((c, i) => ({ ...c, position: { ...c.position, order: i } }));
}

/**
 * 展板管理面板（edit_showcase）。
 * 逐组件编辑：启用开关、顺序（↑↓）、大小（宽度列数 / 最小高度 px）、
 * 组件设置（registry 中的 fields 声明驱动，自动生成表单）。
 */
export default function ShowcaseSettingsPanel({
  layout,
  onChange,
  ctx,
  saving,
  msg,
  onSave,
  onCancel,
}: ShowcaseSettingsPanelProps) {
  const updateConfig = (id: string, patch: Partial<ShowcaseComponentConfig>) => {
    onChange({
      ...layout,
      components: layout.components.map((c) =>
        c.id === id ? { ...c, ...patch } : c,
      ),
    });
  };

  const move = (id: string, direction: -1 | 1) => {
    const comps = [...layout.components];
    const idx = comps.findIndex((c) => c.id === id);
    const target = idx + direction;
    if (idx < 0 || target < 0 || target >= comps.length) return;
    const [item] = comps.splice(idx, 1);
    comps.splice(target, 0, item);
    onChange({ ...layout, components: reindex(comps) });
  };

  const setSetting = (id: string, key: string, value: unknown) => {
    updateConfig(id, {
      settings: { ...layout.components.find((c) => c.id === id)?.settings, [key]: value },
    });
  };

  const toggleId = (id: string, settingKey: string, itemId: string) => {
    const cfg = layout.components.find((c) => c.id === id);
    const list = (cfg?.settings[settingKey] as string[]) ?? [];
    setSetting(
      id,
      settingKey,
      list.includes(itemId)
        ? list.filter((x) => x !== itemId)
        : [...list, itemId],
    );
  };

  const moveId = (
    id: string,
    settingKey: string,
    idx: number,
    direction: -1 | 1,
  ) => {
    const cfg = layout.components.find((c) => c.id === id);
    const list = [...((cfg?.settings[settingKey] as string[]) ?? [])];
    const target = idx + direction;
    if (target < 0 || target >= list.length) return;
    const [item] = list.splice(idx, 1);
    list.splice(target, 0, item);
    setSetting(id, settingKey, list);
  };

  const renderIdsField = (
    cfg: ShowcaseComponentConfig,
    field: Extract<SettingField, { kind: "ids" }>,
  ) => {
    const candidates =
      field.source === "problems" ? ctx.problems : ctx.contests;
    const selectedIds = ((cfg.settings[field.key] as string[]) ?? []).filter(
      (x) => candidates.some((c) => c.id === x),
    );
    const selected = candidates.filter((c) => selectedIds.includes(c.id));
    const unselected = candidates.filter((c) => !selectedIds.includes(c.id));

    return (
      <div className="mt-1">
        <div className="space-y-0.5 max-h-40 overflow-y-auto border border-gray-200 dark:border-gray-700 p-1.5">
          {[...selected, ...unselected].map((item) => {
            const idx = selected.findIndex((s) => s.id === item.id);
            const isSelected = idx >= 0;
            return (
              <div
                key={item.id}
                className="flex items-center gap-2 py-1 px-1 hover:bg-gray-50 dark:hover:bg-gray-800/50"
              >
                <input
                  type="checkbox"
                  checked={isSelected}
                  onChange={() => toggleId(cfg.id, field.key, item.id)}
                  className="accent-gray-800 dark:accent-gray-400"
                />
                <span
                  className={`text-sm flex-1 truncate ${isSelected ? "text-gray-800 dark:text-gray-100" : "text-gray-400 dark:text-gray-500"}`}
                >
                  {isSelected && (
                    <span className="text-gray-400 dark:text-gray-500 mr-1.5 text-xs tabular-nums">
                      {idx + 1}.
                    </span>
                  )}
                  {"title" in item ? item.title : item.name}
                </span>
                {isSelected && (
                  <div className="flex gap-1 shrink-0">
                    <button
                      type="button"
                      onClick={() => moveId(cfg.id, field.key, idx, -1)}
                      disabled={idx === 0}
                      className="text-xs text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 disabled:opacity-30 px-1"
                    >
                      ↑
                    </button>
                    <button
                      type="button"
                      onClick={() => moveId(cfg.id, field.key, idx, 1)}
                      disabled={idx === selected.length - 1}
                      className="text-xs text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 disabled:opacity-30 px-1"
                    >
                      ↓
                    </button>
                  </div>
                )}
              </div>
            );
          })}
        </div>
      </div>
    );
  };

  const renderField = (cfg: ShowcaseComponentConfig, field: SettingField) => {
    switch (field.kind) {
      case "number": {
        const value = cfg.settings[field.key];
        return (
          <label className="flex items-center gap-2 text-sm">
            <span className="text-gray-600 dark:text-gray-300 w-20 shrink-0">
              {field.label}
            </span>
            <input
              type="number"
              min={field.min}
              max={field.max}
              step={field.step ?? 1}
              value={typeof value === "number" ? value : field.min ?? 0}
              onChange={(e) => {
                const n = parseInt(e.target.value, 10);
                setSetting(cfg.id, field.key, Number.isNaN(n) ? (field.min ?? 0) : n);
              }}
              className="w-24 px-2 py-1 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-900 text-sm text-gray-800 dark:text-gray-100"
            />
          </label>
        );
      }
      case "boolean": {
        return (
          <label className="flex items-center gap-2 text-sm cursor-pointer">
            <input
              type="checkbox"
              checked={cfg.settings[field.key] !== false}
              onChange={(e) => setSetting(cfg.id, field.key, e.target.checked)}
              className="accent-gray-800 dark:accent-gray-400"
            />
            <span className="text-gray-600 dark:text-gray-300">{field.label}</span>
          </label>
        );
      }
      case "ids":
        return (
          <div>
            <p className="text-sm text-gray-600 dark:text-gray-300 mb-1">
              {field.label}
            </p>
            {renderIdsField(cfg, field)}
          </div>
        );
    }
  };

  return (
    <section className="mg-box-shadow p-5">
      <h2 className="text-lg font-semibold text-gray-800 dark:text-gray-100 mb-1">
        展板管理
      </h2>
      <p className="text-xs text-gray-500 dark:text-gray-400 mb-4">
        每个区块是一个展板组件：可开关、调整顺序（位置）、设置宽度/最小高度
        （大小）与组件专属设置。宽度在宽屏生效，窄屏自动单列堆叠。
      </p>

      {msg && (
        <div
          className={`mb-3 p-2 text-sm border ${
            msg.includes("失败")
              ? "bg-red-50 border-red-300 text-red-700 dark:bg-red-900/30 dark:border-red-800 dark:text-red-300"
              : "bg-green-50 border-green-300 text-green-700 dark:bg-green-900/30 dark:border-green-800 dark:text-green-300"
          }`}
        >
          {msg}
        </div>
      )}

      <div className="space-y-3">
        {layout.components.map((cfg) => {
          const def = SHOWCASE_COMPONENT_DEFS[cfg.type];
          const width = Math.min(
            SHOWCASE_GRID_COLUMNS,
            Math.max(1, Math.floor(cfg.size?.width ?? SHOWCASE_GRID_COLUMNS)),
          );
          const height =
            typeof cfg.size?.height === "number" && cfg.size.height > 0
              ? cfg.size.height
              : 0;
          const idx = layout.components.findIndex((c) => c.id === cfg.id);

          return (
            <div
              key={cfg.id}
              className="border border-gray-200 dark:border-gray-700 p-3"
            >
              {/* 头部：启用 + 名称 + 顺序 */}
              <div className="flex items-center gap-2 mb-2 flex-wrap">
                <input
                  type="checkbox"
                  checked={cfg.enabled}
                  onChange={(e) => updateConfig(cfg.id, { enabled: e.target.checked })}
                  className="accent-gray-800 dark:accent-gray-400"
                  title="在展板中展示"
                />
                <span className="text-sm font-semibold text-gray-800 dark:text-gray-100">
                  {def ? def.label : `未知组件（${cfg.type}）`}
                </span>
                <span className="text-xs text-gray-400 dark:text-gray-500">
                  {def ? def.description : "该类型尚未注册，保留但不渲染"}
                </span>
                <div className="flex gap-1 ml-auto shrink-0">
                  <button
                    type="button"
                    onClick={() => move(cfg.id, -1)}
                    disabled={idx === 0}
                    className="text-xs text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 disabled:opacity-30 px-1"
                  >
                    ↑
                  </button>
                  <button
                    type="button"
                    onClick={() => move(cfg.id, 1)}
                    disabled={idx === layout.components.length - 1}
                    className="text-xs text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 disabled:opacity-30 px-1"
                  >
                    ↓
                  </button>
                </div>
              </div>

              {/* 大小：宽度 / 高度 */}
              {def && (
                <div className="flex items-center gap-4 mb-2 flex-wrap">
                  <label className="flex items-center gap-2 text-sm">
                    <span className="text-gray-600 dark:text-gray-300 w-12 shrink-0">
                      宽度
                    </span>
                    <select
                      value={width}
                      onChange={(e) =>
                        updateConfig(cfg.id, {
                          size: {
                            ...cfg.size,
                            width: parseInt(e.target.value, 10),
                          },
                        })
                      }
                      className="px-2 py-1 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-900 text-sm text-gray-800 dark:text-gray-100"
                    >
                      {[1, 2, 3, 4].map((w) => (
                        <option key={w} value={w}>
                          {WIDTH_LABELS[w]}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label className="flex items-center gap-2 text-sm">
                    <span className="text-gray-600 dark:text-gray-300 w-20 shrink-0">
                      最小高度
                    </span>
                    <input
                      type="number"
                      min={0}
                      step={10}
                      value={height}
                      onChange={(e) => {
                        const n = parseInt(e.target.value, 10);
                        const h = Number.isNaN(n) || n <= 0 ? undefined : n;
                        updateConfig(cfg.id, {
                          size: h ? { ...cfg.size, height: h } : { width: cfg.size.width },
                        });
                      }}
                      className="w-24 px-2 py-1 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-900 text-sm text-gray-800 dark:text-gray-100"
                    />
                    <span className="text-xs text-gray-400 dark:text-gray-500">
                      px（0 = 自适应）
                    </span>
                  </label>
                </div>
              )}

              {/* 组件设置 */}
              {def && def.fields.length > 0 && (
                <div className="space-y-2 pt-2 border-t border-gray-100 dark:border-gray-800">
                  {def.fields.map((field) => (
                    <div key={field.key}>{renderField(cfg, field)}</div>
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </div>

      <div className="flex gap-3 items-center mt-4">
        <button
          onClick={onSave}
          disabled={saving}
          className="mg-btn mg-btn-primary mg-btn-md"
        >
          {saving ? "保存中..." : "保存展板"}
        </button>
        <button
          onClick={onCancel}
          className="px-5 py-2 text-sm border border-gray-300 dark:border-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800"
        >
          取消
        </button>
      </div>
    </section>
  );
}