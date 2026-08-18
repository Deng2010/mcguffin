import { useState, useEffect } from "react";
import { useSiteStore } from "../../../stores/siteStore";
import MarkdownRenderer from "../../../components/MarkdownRenderer";
import MarkdownEditor from "../../../components/MarkdownEditor";
import { useToast } from "../../../errors/ToastContext";
import type { ShowcaseComponentProps } from "../types";

/**
 * 展板组件：团队简介
 * 展示站点名称与团队介绍；管理员可内联编辑简介。
 */
export default function IntroComponent({
  ctx,
}: ShowcaseComponentProps) {
  const toast = useToast();
  const { siteInfo, updateDescription } = useSiteStore();

  const [editing, setEditing] = useState(false);
  const [draftDescription, setDraftDescription] = useState("");

  // 打开编辑时同步草稿
  useEffect(() => {
    if (editing && siteInfo) {
      setDraftDescription(siteInfo.description);
    }
  }, [editing, siteInfo]);

  const handleSave = async () => {
    const res = await updateDescription(draftDescription);
    if (!res.success) {
      toast.error(res.message);
      return;
    }
    setEditing(false);
  };

  return (
    <section className="mg-box-shadow p-6 h-full">
      <div className="flex items-center justify-between mb-3">
        <h1 className="text-2xl font-bold text-gray-800 dark:text-gray-100">
          {ctx.siteInfo?.name || siteInfo?.name || "McGuffin"}
        </h1>
        {ctx.isAdmin && !editing && (
          <button
            onClick={() => setEditing(true)}
            className="text-xs px-3 py-1 border border-gray-300 dark:border-gray-700 text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800"
          >
            编辑简介
          </button>
        )}
      </div>

      {editing ? (
        <div className="space-y-3">
          <MarkdownEditor
            value={draftDescription}
            onChange={setDraftDescription}
            placeholder="在此输入团队简介..."
            rows={20}
          />
          <div className="flex gap-2">
            <button
              onClick={handleSave}
              className="mg-btn mg-btn-primary mg-btn-sm"
            >
              保存
            </button>
            <button
              onClick={() => setEditing(false)}
              className="mg-btn mg-btn-ghost mg-btn-sm"
            >
              取消
            </button>
          </div>
        </div>
      ) : (
        <MarkdownRenderer
          content={
            ctx.siteInfo?.description ||
            (ctx.isAdmin
              ? '<span class="text-gray-300 italic dark:text-gray-600">点击「编辑简介」添加团队介绍</span>'
              : '<span class="text-gray-300 italic dark:text-gray-600">暂无团队简介</span>')
          }
        />
      )}
    </section>
  );
}