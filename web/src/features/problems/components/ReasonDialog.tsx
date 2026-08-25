interface ReasonDialogState {
  open: boolean;
  problemId: string;
  action: string;
}

interface ReasonDialogProps {
  dialog: ReasonDialogState;
  reasonText: string;
  onReasonTextChange: (v: string) => void;
  onCancel: () => void;
  onConfirm: () => void;
}

export default function ReasonDialog({
  dialog,
  reasonText,
  onReasonTextChange,
  onCancel,
  onConfirm,
}: ReasonDialogProps) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30">
      <div
        className="bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 shadow-lg p-6 w-full max-w-md"
        onClick={(e) => e.stopPropagation()}
      >
        <h3 className="text-lg font-semibold text-gray-800 dark:text-gray-100 mb-4">
          {dialog.action === "reply" ? "回复建议" : "退回理由"}
        </h3>
        <p className="text-sm text-gray-500 dark:text-gray-400 mb-3">
          {dialog.action === "reply"
            ? "请输入对题目的建议，系统将通知出题人："
            : dialog.action === "reject"
              ? "请输入退回理由（不少于 10 个字），系统将通知出题人："
              : "请输入退回理由（可选），系统将通知出题人："}
        </p>
        <textarea
          value={reasonText}
          onChange={(e) => onReasonTextChange(e.target.value)}
          rows={4}
          placeholder={
            dialog.action === "reply"
              ? "如：请补充数据范围说明..."
              : "如：本题描述不够清晰，请补充题解后再提交..."
          }
          className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 focus:outline-none focus:border-gray-500 text-sm mb-4"
          autoFocus
        />
        {dialog.action === "reject" && reasonText.trim().length > 0 && (
          <p
            className={`text-xs mb-3 ${reasonText.trim().length >= 10 ? "text-green-600 dark:text-green-400" : "text-red-500 dark:text-red-400"}`}
          >
            已输入 {reasonText.trim().length} / 10 字
          </p>
        )}
        <div className="flex justify-end gap-3">
          <button
            onClick={onCancel}
            className="px-4 py-2 border border-gray-300 dark:border-gray-700 text-gray-600 dark:text-gray-300 text-sm hover:bg-gray-100 dark:hover:bg-gray-800"
          >
            取消
          </button>
          <button
            onClick={onConfirm}
            disabled={dialog.action === "reply" && !reasonText.trim()}
            className={`px-4 py-2 text-sm text-white disabled:opacity-50 ${
              dialog.action === "reply"
                ? "bg-blue-600 hover:bg-blue-500"
                : dialog.action === "reject"
                  ? "bg-red-600 hover:bg-red-500"
                  : "bg-yellow-600 hover:bg-yellow-500"
            }`}
          >
            确认{dialog.action === "reply" ? "回复" : "退回"}
          </button>
        </div>
      </div>
    </div>
  );
}
