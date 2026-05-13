import { useState, useRef, useCallback, useEffect, useMemo } from 'react'
import MarkdownRenderer from './MarkdownRenderer'

interface MarkdownEditorProps {
  value: string
  onChange: (value: string) => void
  label?: string
  placeholder?: string
  /** 编辑框固定行数（默认 30） */
  rows?: number
  disabled?: boolean
  required?: boolean
  /** 在 label 旁显示 "— {optionalNote}" */
  optionalNote?: string
  /** 键盘事件（如 Ctrl+Enter 提交） */
  onKeyDown?: (e: React.KeyboardEvent<HTMLTextAreaElement>) => void
  /** 在内容变化时额外回调，传递光标位置（用于 @mention 检测） */
  onCursorChange?: (value: string, cursorPos: number) => void
}

const FONT_FAMILY = "'JetBrains Mono', monospace"
const LH = 1.5 // line-height (rem)
const PY = 0.5 // padding-y (rem)
const PX = 0.75 // padding-x (rem)
const FS = 0.875 // font-size (rem)

/** 跨越一个方向：将 source 的滚动比例应用到 target */
function syncScroll(source: HTMLElement, target: HTMLElement | null) {
  if (!target) return
  const sRange = source.scrollHeight - source.clientHeight
  const tRange = target.scrollHeight - target.clientHeight
  if (sRange > 0 && tRange > 0) {
    target.scrollTop = (source.scrollTop / sRange) * tRange
  }
}

export default function MarkdownEditor({
  value,
  onChange,
  label,
  placeholder = '在此输入 Markdown...',
  rows = 30,
  disabled = false,
  required = false,
  optionalNote,
  onKeyDown,
  onCursorChange,
}: MarkdownEditorProps) {
  const [showPreview, setShowPreview] = useState(true)
  const taRef = useRef<HTMLTextAreaElement>(null)
  const guRef = useRef<HTMLDivElement>(null)
  const wrapRef = useRef<HTMLDivElement>(null)
  const prevRef = useRef<HTMLDivElement>(null)

  const heightRem = rows * LH + PY * 2

  const lineCount = useMemo(
    () => (value.match(/\n/g) || []).length + 1,
    [value],
  )

  const gutterWidth = useMemo(
    () => Math.max(2.5, String(lineCount).length * 0.7 + 1.2),
    [lineCount],
  )

  // ── Editor scroll → gutter + preview ──
  const onTAScroll = useCallback(() => {
    const ta = taRef.current
    if (!ta) return
    if (guRef.current) guRef.current.scrollTop = ta.scrollTop
    syncScroll(ta, prevRef.current)
  }, [])

  // ── Keep preview height in step with editor ──
  useEffect(() => {
    if (!showPreview) return
    const wrap = wrapRef.current
    const box = prevRef.current
    if (!wrap || !box) return
    const sync = () => {
      const h = wrap.getBoundingClientRect().height
      if (h > 0) box.style.maxHeight = box.style.minHeight = `${h}px`
    }
    sync()
    const ro = new ResizeObserver(sync)
    ro.observe(wrap)
    return () => ro.disconnect()
  }, [showPreview])

  const mono: React.CSSProperties = {
    fontFamily: FONT_FAMILY,
    fontSize: `${FS}rem`,
    lineHeight: LH,
  }

  const padStyle: React.CSSProperties = {
    ...mono,
    padding: `${PY}rem ${PX}rem`,
    whiteSpace: 'pre' as const,
    tabSize: 4,
    MozTabSize: 4,
  }

  return (
    <div className="mb-4">
      {/* Label + toggle */}
      <div className="flex items-center justify-between mb-2">
        {label ? (
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-200">
            {label}
            {optionalNote !== undefined && (
              <span className="text-gray-400 dark:text-gray-500 font-normal ml-1">— {optionalNote}</span>
            )}
          </label>
        ) : (
          <span />
        )}
        <button
          type="button"
          onClick={() => setShowPreview(v => !v)}
          className="text-xs px-2 py-1 border border-gray-300 dark:border-gray-700 text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800 focus:outline-none select-none"
        >
          {showPreview ? '收起预览 ▸' : '展开预览 ▸'}
        </button>
      </div>

      <div className="flex gap-0">
        {/* ── Editor panel ── */}
        <div
          ref={wrapRef}
          className={`flex border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 ${
            showPreview ? 'w-1/2 border-r-0' : 'w-full'
          }`}
          style={{ height: `${heightRem}rem`, overflow: 'hidden' }}
        >
          {/* Line number gutter */}
          <div
            ref={guRef}
            className="flex-shrink-0 overflow-hidden select-none text-right border-r border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900"
            style={{
              ...mono,
              width: `${gutterWidth}rem`,
              paddingTop: `${PY}rem`,
              whiteSpace: 'pre',
              color: '#9ca3af',
            }}
          >
            <div style={{ paddingRight: '0.4rem', paddingBottom: `${heightRem}rem` }}>
              {Array.from({ length: lineCount }, (_, i) => (
                <div key={i}>{i + 1}</div>
              ))}
            </div>
          </div>

          {/* Textarea */}
          <textarea
            ref={taRef}
            value={value}
            onChange={e => {
              onChange(e.target.value)
              onCursorChange?.(e.target.value, e.target.selectionStart)
            }}
            onScroll={onTAScroll}
            onKeyDown={onKeyDown}
            disabled={disabled}
            required={required}
            placeholder={placeholder}
            spellCheck={false}
            className="flex-1 bg-transparent focus:outline-none resize-none overflow-auto"
            style={{ ...padStyle, border: 'none' }}
          />
        </div>

        {/* ── Preview panel ── */}
        {showPreview && (
          <div
            ref={prevRef}
            className="w-1/2 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-900 overflow-hidden"
            style={{ height: `${heightRem}rem` }}
          >
            <div className="px-4 py-2 border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800">
              <span className="text-xs font-medium text-gray-500 dark:text-gray-400">预览</span>
            </div>
            <div className="p-4">
              {value.trim() ? (
                <MarkdownRenderer content={value} />
              ) : (
                <p className="text-sm text-gray-400 dark:text-gray-500 italic">暂无内容</p>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
