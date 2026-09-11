import { useState, useEffect, useCallback, useRef } from "react";
import type { PluginUserInfo, PluginTeamMember } from "./data";
import * as data from "./data";
import { usePluginContext } from "./PluginContext";

// ── Context: plugin id resolution ──

/**
 * 当前插件 id。
 *
 * 优先从插件上下文（`PluginPage` / `PluginSlots` 注入）读取 —— 这是可靠来源，
 * 因为路由路径与插件 id 并不总是一致（如 id=lollipop-rank 挂在 /plugins/lollipop）。
 * 仅在没有上下文时（例如插件组件被宿主之外的地方直接渲染）回退解析 URL。
 */
export function usePluginId(): string {
  const ctx = usePluginContext();
  const fallbackRef = useRef<string>("");
  if (!fallbackRef.current) {
    const match = window.location.pathname.match(/\/plugins\/([^/]+)/);
    if (match) fallbackRef.current = match[1];
  }
  return ctx?.pluginId ?? fallbackRef.current;
}

// ── Data hooks ──

export function usePluginData<T = string>(
  pluginId: string,
  namespace: string,
  key: string,
  options?: { defaultValue?: T },
) {
  const [value, setValue] = useState<T | null>(options?.defaultValue ?? null);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const raw = await data.getPluginData(pluginId, namespace, key);
      setValue((raw ? JSON.parse(raw) : options?.defaultValue) as T);
    } catch {
      setValue(options?.defaultValue ?? null);
    }
    setLoading(false);
  }, [pluginId, namespace, key]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { value, loading, refresh, setValue };
}

// ── Counter hooks ──

export function usePluginCounter(
  pluginId: string,
  namespace: string,
  key: string,
) {
  const [count, setCount] = useState(0);

  const add = useCallback(
    async (delta: number) => {
      const val = await data.pluginAdd(pluginId, namespace, key, delta);
      setCount(val);
      return val;
    },
    [pluginId, namespace, key],
  );

  return { count, add };
}

// ── Set hooks ──

export function usePluginSet(pluginId: string, namespace: string, key: string) {
  const [members, setMembers] = useState<string[]>([]);

  const refresh = useCallback(async () => {
    try {
      const m = await data.pluginSetMembers(pluginId, namespace, key);
      setMembers(m);
    } catch {
      /* ignore */
    }
  }, [pluginId, namespace, key]);

  const add = useCallback(
    async (member: string) => {
      const added = await data.pluginSetAdd(pluginId, namespace, key, member);
      if (added) setMembers((prev) => [...prev, member]);
      return added;
    },
    [pluginId, namespace, key],
  );

  const remove = useCallback(
    async (member: string) => {
      const removed = await data.pluginSetRemove(
        pluginId,
        namespace,
        key,
        member,
      );
      if (removed) setMembers((prev) => prev.filter((m) => m !== member));
      return removed;
    },
    [pluginId, namespace, key],
  );

  const isMember = useCallback(
    async (member: string) => {
      return data.pluginSetIsMember(pluginId, namespace, key, member);
    },
    [pluginId, namespace, key],
  );

  return { members, add, remove, isMember, refresh };
}

// ── Keys hook ──

export function usePluginKeys(
  pluginId: string,
  namespace: string,
  prefix?: string,
) {
  const [keys, setKeys] = useState<string[]>([]);

  const refresh = useCallback(async () => {
    try {
      const k = await data.pluginKeys(pluginId, namespace, prefix);
      setKeys(k);
    } catch {
      /* ignore */
    }
  }, [pluginId, namespace, prefix]);

  return { keys, refresh };
}

// ── User info hooks ──

export function usePluginUserMe(pluginId: string) {
  const [user, setUser] = useState<PluginUserInfo | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const u = await data.pluginUserMe(pluginId);
      setUser(u);
    } catch {
      /* ignore */
    }
    setLoading(false);
  }, [pluginId]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { user, loading, refresh };
}

export function usePluginUser(pluginId: string, userId: string) {
  const [user, setUser] = useState<PluginUserInfo | null>(null);

  useEffect(() => {
    data
      .pluginUserGet(pluginId, userId)
      .then(setUser)
      .catch(() => {});
  }, [pluginId, userId]);

  return user;
}

export function usePluginTeamMembers(pluginId: string) {
  const [members, setMembers] = useState<PluginTeamMember[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await data.pluginUserList(pluginId);
      setMembers(res.members ?? []);
    } catch (err: any) {
      const detail = err?.responseText
        ? (() => { try { return JSON.parse(err.responseText).message; } catch { return err.responseText; } })()
        : err?.message || String(err);
      setError(detail);
      setMembers([]);
    }
    setLoading(false);
  }, [pluginId]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { members, loading, error, refresh };
}
