import { useContext } from "react";
import { ConfigCtx, inputClass } from "../config-context";

export default function ServerSection() {
  const c = useContext(ConfigCtx);
  return (
    <div className="space-y-4">
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">
          站点 URL
        </label>
        <input
          type="text"
          value={c.siteUrl}
          onChange={(e) => c.setSiteUrl(e.target.value)}
          className={inputClass}
          placeholder="https://lba-oi.team"
        />
      </div>
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">
          端口
        </label>
        <input
          type="number"
          value={c.port}
          onChange={(e) => c.setPort(e.target.value)}
          className={inputClass}
          placeholder="3000"
        />
      </div>
    </div>
  );
}
