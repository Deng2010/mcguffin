import { useCallback, useState, type CSSProperties } from "react";

/**
 * 示例页面组件：演示插件私有键值存储（KV）的读写。
 *
 * 本文件直接 `import { useState } from "react"` 并使用 JSX —— 宿主在
 * `web/index.html` 提供了 import map，把 `react` / `react/jsx-runtime` 映射到
 * `/plugin-sdk/*.js` 垫片，垫片再转发到宿主那**同一份** React 实例，
 * 因此插件既不用打包 React，也不会出现两份实例。
 *
 * 插件的 SDK 能力（KV / 用户 / 团队…）仍从全局取：
 *   const { usePluginData } = window.__MCGUFFIN_SDK__;
 * 数据接口需要 `storage` 权限（在 plugin.json 的 permissions_needed 中声明）。
 *
 * 插件 id 用 `usePluginId()` 从宿主上下文取，不要自己解析 URL。
 */
const { usePluginData, usePluginId, setPluginData } = window.__MCGUFFIN_SDK__;

/** 存在 KV 里的数据结构（单值 ≤ 64 KiB，自行做 JSON 序列化）。 */
interface PluginState {
  count: number;
  note: string;
  updatedAt: string;
}

const STATE_NAMESPACE = "state";
const STATE_KEY = "main";
const DEFAULTS: PluginState = { count: 0, note: "", updatedAt: "" };

const styles: Record<string, CSSProperties> = {
  page: { padding: "1.5rem", maxWidth: "40rem" },
  title: { fontSize: "1.25rem", fontWeight: 600, marginBottom: "0.5rem" },
  hint: { fontSize: "0.8125rem", opacity: 0.7, marginBottom: "0.75rem" },
  row: {
    display: "flex",
    gap: "0.5rem",
    alignItems: "center",
    marginTop: "0.75rem",
  },
  button: {
    padding: "0.375rem 0.75rem",
    border: "1px solid currentColor",
    borderRadius: "0.375rem",
    cursor: "pointer",
  },
  input: {
    padding: "0.375rem 0.5rem",
    border: "1px solid currentColor",
    borderRadius: "0.375rem",
    flex: 1,
  },
};

export default function Page() {
  const pluginId = usePluginId();
  const { value, loading, refresh } = usePluginData<PluginState>(
    pluginId,
    STATE_NAMESPACE,
    STATE_KEY,
    { defaultValue: DEFAULTS },
  );
  const [draft, setDraft] = useState("");
  const [saving, setSaving] = useState(false);

  const state = value ?? DEFAULTS;

  const save = useCallback(
    async (next: PluginState) => {
      setSaving(true);
      try {
        await setPluginData(
          pluginId,
          STATE_NAMESPACE,
          STATE_KEY,
          JSON.stringify(next),
        );
        await refresh();
      } finally {
        setSaving(false);
      }
    },
    [pluginId, refresh],
  );

  const touch = (next: PluginState): PluginState => ({
    ...next,
    updatedAt: new Date().toISOString(),
  });

  return (
    <div style={styles.page}>
      <h1 style={styles.title}>我的插件</h1>
      <p style={styles.hint}>
        pluginId = {pluginId}；数据保存在插件私有 KV（需要 storage 权限）
      </p>

      {loading ? (
        <p style={styles.hint}>加载中…</p>
      ) : (
        <div>
          <p>计数：{state.count}</p>
          <p>备注：{state.note || "（空）"}</p>
          <p style={styles.hint}>更新时间：{state.updatedAt || "—"}</p>

          <div style={styles.row}>
            <button
              style={styles.button}
              disabled={saving}
              onClick={() =>
                void save(touch({ ...state, count: state.count + 1 }))
              }
            >
              计数 +1
            </button>
            <button
              style={styles.button}
              disabled={loading || saving}
              onClick={() => void refresh()}
            >
              重新读取
            </button>
          </div>

          <div style={styles.row}>
            <input
              style={styles.input}
              value={draft}
              placeholder="写点备注…"
              onChange={(e) => setDraft(e.target.value)}
            />
            <button
              style={styles.button}
              disabled={saving || draft.length === 0}
              onClick={() => {
                void save(touch({ ...state, note: draft }));
                setDraft("");
              }}
            >
              保存备注
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
