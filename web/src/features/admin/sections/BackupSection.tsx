import { useContext } from "react";
import { ConfigCtx, inputClass } from "../config-context";

export default function BackupSection() {
  const c = useContext(ConfigCtx);
  return (
    <div className="space-y-4">
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">
          自动备份间隔（分钟）
        </label>
        <input
          type="number"
          min={10}
          max={1440}
          value={c.backupInterval}
          onChange={(e) => c.setBackupInterval(parseInt(e.target.value) || 60)}
          className={inputClass}
        />
        <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">
          每隔多少分钟自动备份一次。最小值 10 分钟，最大值 1440 分钟（24
          小时）。
        </p>
      </div>
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">
          最大备份保留数量
        </label>
        <input
          type="number"
          min={1}
          max={999}
          value={c.backupRetention}
          onChange={(e) =>
            c.setBackupRetention(parseInt(e.target.value) || 48)
          }
          className={inputClass}
        />
        <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">
          最多保留多少个自动备份文件。超出数量的旧备份会被自动清理。
        </p>
      </div>
      <div>
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-200 mb-1">
          备份目录（留空使用默认位置）
        </label>
        <input
          type="text"
          value={c.backupDirectory}
          onChange={(e) => c.setBackupDirectory(e.target.value)}
          className={inputClass}
          placeholder="留空则使用 data 目录下的 backups/"
        />
        <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">
          自定义备份文件、导出数据、导入数据的存放目录。修改后立即生效。
        </p>
      </div>
      <p className="text-xs text-gray-400 dark:text-gray-500">
        备份间隔和保留数量修改后需重启服务生效。
      </p>
    </div>
  );
}
