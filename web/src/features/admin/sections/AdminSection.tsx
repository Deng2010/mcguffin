import { useContext } from "react";
import { ConfigCtx, inputClass } from "../config-context";

export default function AdminSection() {
  const c = useContext(ConfigCtx);
  const passwordEmpty = !c.adminPassword.trim();
  return (
    <div className="space-y-4">
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">
          登录密码
        </label>
        <input
          type="password"
          value={c.adminPassword}
          onChange={(e) => c.setAdminPassword(e.target.value)}
          className={`${inputClass} ${passwordEmpty ? "border-red-500 dark:border-red-500" : ""}`}
        />
        {passwordEmpty && (
          <p className="text-xs text-red-500 dark:text-red-400 mt-1">
            管理员密码不能为空
          </p>
        )}
        <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">
          修改后需重启服务生效
        </p>
      </div>
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">
          显示名称
        </label>
        <input
          type="text"
          value={c.displayName}
          onChange={(e) => c.setDisplayName(e.target.value)}
          className={inputClass}
        />
      </div>
    </div>
  );
}
