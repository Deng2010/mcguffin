import { useContext } from "react";
import { ConfigCtx, inputClass } from "../config-context";

export default function OAuthSection() {
  const c = useContext(ConfigCtx);
  return (
    <div className="space-y-4">
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">
          Client ID
        </label>
        <input
          type="text"
          value={c.cpClientId}
          onChange={(e) => c.setCpClientId(e.target.value)}
          className={inputClass}
        />
      </div>
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">
          Client Secret
        </label>
        <input
          type="text"
          value={c.cpClientSecret}
          onChange={(e) => c.setCpClientSecret(e.target.value)}
          className={inputClass}
        />
      </div>
    </div>
  );
}
