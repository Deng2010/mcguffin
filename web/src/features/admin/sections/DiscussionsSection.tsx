import { useState, useContext } from "react";
import { ConfigCtx } from "../config-context";

export default function DiscussionsSection() {
  const c = useContext(ConfigCtx);
  const [newTagName, setNewTagName] = useState("");
  const [newTagColor, setNewTagColor] = useState("#6366f1");
  const [newTagDesc, setNewTagDesc] = useState("");
  const [newEmojiName, setNewEmojiName] = useState("");
  const [newEmojiChar, setNewEmojiChar] = useState("");

  const addTag = () => {
    const name = newTagName.trim();
    if (!name || name in c.discussionTags) return;
    c.setDiscussionTags({
      ...c.discussionTags,
      [name]: { color: newTagColor, description: newTagDesc.trim() },
    });
    setNewTagName("");
    setNewTagColor("#6366f1");
    setNewTagDesc("");
  };

  const removeTag = (name: string) => {
    const { [name]: _, ...rest } = c.discussionTags;
    c.setDiscussionTags(rest);
  };

  const addEmoji = () => {
    const name = newEmojiName.trim();
    if (!name || !newEmojiChar.trim() || name in c.discussionEmojis) return;
    c.setDiscussionEmojis({
      ...c.discussionEmojis,
      [name]: { char: newEmojiChar.trim() },
    });
    setNewEmojiName("");
    setNewEmojiChar("");
  };

  const removeEmoji = (name: string) => {
    const { [name]: _, ...rest } = c.discussionEmojis;
    c.setDiscussionEmojis(rest);
  };

  return (
    <>
      <section className="mb-10">
        <h2 className="text-lg font-semibold text-gray-800 dark:text-gray-100 mb-4">
          标签管理
        </h2>
        <p className="text-xs text-gray-500 dark:text-gray-400 mb-3">
          添加或删除讨论区标签。保存后立即生效。
        </p>
        <div className="flex flex-wrap items-end gap-3 mb-4 p-3 bg-gray-50 dark:bg-gray-800/50 border border-gray-300 dark:border-gray-700">
          <div>
            <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">
              名称
            </label>
            <input
              type="text"
              value={newTagName}
              onChange={(e) => setNewTagName(e.target.value)}
              placeholder="标签名"
              className="w-28 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-gray-800 dark:text-gray-100 px-2 py-1.5 text-sm"
              onKeyDown={(e) => e.key === "Enter" && addTag()}
            />
          </div>
          <div>
            <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">
              颜色
            </label>
            <input
              type="color"
              value={newTagColor}
              onChange={(e) => setNewTagColor(e.target.value)}
              className="w-10 h-8 p-0.5 border border-gray-300 dark:border-gray-700 cursor-pointer bg-white dark:bg-gray-800"
            />
          </div>
          <div>
            <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">
              备注
            </label>
            <input
              type="text"
              value={newTagDesc}
              onChange={(e) => setNewTagDesc(e.target.value)}
              placeholder="可选备注"
              className="w-36 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-gray-800 dark:text-gray-100 px-2 py-1.5 text-sm"
              onKeyDown={(e) => e.key === "Enter" && addTag()}
            />
          </div>
          <button
            onClick={addTag}
            className="px-3 py-1.5 bg-gray-800 text-white text-sm hover:bg-gray-700 dark:bg-gray-700 dark:hover:bg-gray-600"
          >
            添加
          </button>
        </div>
        <div className="space-y-1">
          {Object.keys(c.discussionTags).length === 0 && (
            <p className="text-sm text-gray-400 dark:text-gray-500">暂无标签</p>
          )}
          {Object.entries(c.discussionTags).map(([name, fields]) => (
            <div
              key={name}
              className="flex items-center gap-3 px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700"
            >
              <span
                className="w-2.5 h-2.5 inline-block shrink-0"
                style={{ backgroundColor: fields.color }}
              />
              <span className="text-sm text-gray-800 dark:text-gray-100 w-24">
                {name}
              </span>
              <span className="text-xs text-gray-400 dark:text-gray-500 flex-1">
                {fields.description}
              </span>
              <button
                onClick={() => removeTag(name)}
                className="text-xs text-red-500 hover:text-red-700 dark:text-red-400 dark:hover:text-red-300"
              >
                删除
              </button>
            </div>
          ))}
        </div>
      </section>

      <section>
        <h2 className="text-lg font-semibold text-gray-800 dark:text-gray-100 mb-4">
          表情管理
        </h2>
        <div className="flex flex-wrap items-end gap-3 mb-4 p-3 bg-gray-50 dark:bg-gray-800/50 border border-gray-300 dark:border-gray-700">
          <div>
            <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">
              标识
            </label>
            <input
              type="text"
              value={newEmojiName}
              onChange={(e) => setNewEmojiName(e.target.value)}
              placeholder="如：fire"
              className="w-24 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-gray-800 dark:text-gray-100 px-2 py-1.5 text-sm"
              onKeyDown={(e) => e.key === "Enter" && addEmoji()}
            />
          </div>
          <div>
            <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">
              字符
            </label>
            <input
              type="text"
              value={newEmojiChar}
              onChange={(e) => setNewEmojiChar(e.target.value)}
              placeholder="如：🔥"
              maxLength={2}
              className="w-16 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-gray-800 dark:text-gray-100 px-2 py-1.5 text-sm text-center"
              onKeyDown={(e) => e.key === "Enter" && addEmoji()}
            />
          </div>
          {newEmojiChar && (
            <div className="text-2xl leading-none pb-1">{newEmojiChar}</div>
          )}
          <button
            onClick={addEmoji}
            className="px-3 py-1.5 bg-gray-800 text-white text-sm hover:bg-gray-700 dark:bg-gray-700 dark:hover:bg-gray-600"
          >
            添加
          </button>
        </div>
        <div className="space-y-1">
          {Object.keys(c.discussionEmojis).length === 0 && (
            <p className="text-sm text-gray-400 dark:text-gray-500">暂无表情</p>
          )}
          {Object.entries(c.discussionEmojis).map(([name, fields]) => (
            <div
              key={name}
              className="flex items-center gap-3 px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700"
            >
              <span className="text-xl w-8 text-center shrink-0">
                {fields.char}
              </span>
              <span className="text-sm text-gray-800 dark:text-gray-100 flex-1">
                {name}
              </span>
              <button
                onClick={() => removeEmoji(name)}
                className="text-xs text-red-500 hover:text-red-700 dark:text-red-400 dark:hover:text-red-300"
              >
                删除
              </button>
            </div>
          ))}
        </div>
      </section>
    </>
  );
}
