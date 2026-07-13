import { useContext } from "react";
import { ConfigCtx, inputClass } from "../config-context";

export default function SiteSection() {
  const c = useContext(ConfigCtx);
  return (
    <div className="space-y-4">
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">
          站点名称
        </label>
        <input
          type="text"
          value={c.siteName}
          onChange={(e) => c.setSiteName(e.target.value)}
          className={inputClass}
          placeholder="McGuffin"
        />
      </div>
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">
          网页标题
        </label>
        <input
          type="text"
          value={c.siteTitle}
          onChange={(e) => c.setSiteTitle(e.target.value)}
          className={inputClass}
          placeholder="与站点名称相同"
        />
      </div>
    </div>
  );
}
