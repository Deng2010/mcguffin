import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import remarkMath from 'remark-math'
import rehypeKatex from 'rehype-katex'
import rehypeRaw from 'rehype-raw'
import rehypePrism from 'rehype-prism-plus'
import 'katex/dist/katex.min.css'
import 'prismjs/themes/prism-tomorrow.css'
import type { Components } from 'react-markdown'

interface Props {
  content: string
  className?: string
}

// ============== Luogu Markdown Preprocessing ==============

/**
 * Pre-process Luogu-specific Markdown syntax into standard HTML
 * before passing to ReactMarkdown.
 *
 * Features implemented:
 * - `:::type[title]...:::` or `::::type[title]...::::` → collapsible callout blocks (info, warning, error, success)
 * - `:::align{center}...:::` → center-aligned block
 * - `:::align{right}...:::` → right-aligned block
 * - `:::epigraph[—source]...:::` → epigraph quote block
 */
function preprocessLuoguMarkdown(md: string): string {
  let result = md

  const calloutColors: Record<string, { bg: string; border: string; text: string; summaryBg: string; icon: string }> = {
    info:    { bg: 'bg-blue-50',   border: 'border-blue-300', text: 'text-blue-800', summaryBg: 'bg-blue-100', icon: 'ℹ' },
    warning: { bg: 'bg-yellow-50', border: 'border-yellow-300', text: 'text-yellow-800', summaryBg: 'bg-yellow-100', icon: '⚠' },
    error:   { bg: 'bg-red-50',    border: 'border-red-300', text: 'text-red-800', summaryBg: 'bg-red-100', icon: '✕' },
    success: { bg: 'bg-green-50',  border: 'border-green-300', text: 'text-green-800', summaryBg: 'bg-green-100', icon: '✓' },
  }

  // 1. Handle :::type[title]...::: or ::::type[title]...:::: (collapsible callout with title)
  result = result.replace(
    /:{3,4}(info|warning|error|success)\[((?:[^\[\]]|\[[^\]]*\])*)\](?:\{(\w+)\})?\s*\n([\s\S]*?):{3,4}/gm,
    (_, type, title, attr, content) => {
      const c = calloutColors[type] || calloutColors.info
      const isOpen = attr === 'open'
      return `<details${isOpen ? ' open' : ''} class="luogu-callout ${c.border} rounded mb-3"><summary class="${c.summaryBg} ${c.text} px-3 py-2 text-sm font-semibold cursor-pointer hover:opacity-80 select-none rounded-t">${c.icon} ${title}</summary><div class="${c.bg} ${c.text} p-3 border-t-0 rounded-b">\n\n${content}\n\n</div></details>`
    }
  )

  // 2. Handle :::align{center}...::: (also ::::align{center}...::::)
  result = result.replace(
    /:{3,4}align\{(center|right)\}\s*\n([\s\S]*?):{3,4}/g,
    (_, align, content) => {
      return `<div class="luogu-align luogu-align-${align}">\n\n${content}\n\n</div>`
    }
  )

  // 3. Handle :::epigraph[source]...::: (also ::::epigraph[source]...::::)
  result = result.replace(
    /:{3,4}epigraph\[((?:[^\[\]]|\[[^\]]*\])*)\]\s*\n([\s\S]*?):{3,4}/g,
    (_, source, content) => {
      return `<blockquote class="luogu-epigraph border-l-4 border-gray-400 pl-4 mb-3"><div>\n\n${content}\n\n</div>${source ? `<div class="text-right text-sm text-gray-500 mt-1">${source}</div>` : ''}</blockquote>`
    }
  )

  // 4. Handle :::type without title (bare callout)
  result = result.replace(
    /:{3,4}(info|warning|error|success)\s*\n([\s\S]*?):{3,4}/g,
    (_, type, content) => {
      const c = calloutColors[type] || calloutColors.info
      return `<details class="luogu-callout ${c.border} rounded mb-3"><summary class="${c.summaryBg} ${c.text} px-3 py-2 text-sm font-semibold cursor-pointer hover:opacity-80 select-none rounded-t">${c.icon} ${type === 'info' ? '信息' : type === 'warning' ? '警告' : type === 'error' ? '错误' : '成功'}</summary><div class="${c.bg} ${c.text} p-3 border-t-0 rounded-b">\n\n${content}\n\n</div></details>`
    }
  )

  return result
}

// Languages that are NOT programming languages — get word-wrapping instead of horizontal scroll
const wrapLanguages = new Set(['', 'text', 'plain', 'plaintext', 'txt', 'console', 'output', 'stdout', 'none'])

// ============== Custom Components ==============

const components: Components = {
  h1: ({ children }) => (
    <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100 mt-6 mb-4 pb-2 border-b border-gray-200 dark:border-gray-700">{children}</h1>
  ),
  h2: ({ children }) => (
    <h2 className="text-xl font-bold text-gray-800 dark:text-gray-100 mt-5 mb-3">{children}</h2>
  ),
  h3: ({ children }) => (
    <h3 className="text-lg font-semibold text-gray-800 dark:text-gray-100 mt-4 mb-2">{children}</h3>
  ),
  h4: ({ children }) => (
    <h4 className="text-base font-semibold text-gray-700 dark:text-gray-200 mt-3 mb-1">{children}</h4>
  ),
  h5: ({ children }) => (
    <h5 className="text-sm font-semibold text-gray-600 dark:text-gray-300 mt-2 mb-1">{children}</h5>
  ),
  h6: ({ children }) => (
    <h6 className="text-xs font-semibold text-gray-500 dark:text-gray-400 mt-2 mb-1">{children}</h6>
  ),
  p: ({ children }) => (
    <p className="text-sm text-gray-700 dark:text-gray-300 mb-3 leading-relaxed">{children}</p>
  ),
  ul: ({ children }) => (
    <ul className="list-disc pl-5 mb-3 space-y-1 text-sm text-gray-700 dark:text-gray-300">{children}</ul>
  ),
  ol: ({ children }) => (
    <ol className="list-decimal pl-5 mb-3 space-y-1 text-sm text-gray-700 dark:text-gray-300">{children}</ol>
  ),
  li: ({ children }) => (
    <li className="text-sm text-gray-700 dark:text-gray-300">{children}</li>
  ),
  a: ({ href, children }) => (
    <a href={href} target="_blank" rel="noopener noreferrer" className="text-blue-600 dark:text-blue-400 underline hover:text-blue-800 dark:hover:text-blue-300">
      {children}
    </a>
  ),
  code: ({ className, children, ...props }) => {
    // Inline code: no className
    if (!className) {
      return <code className="bg-gray-100 dark:bg-gray-800 text-red-600 dark:text-red-400 px-1 py-0.5 text-xs rounded">{children}</code>
    }
    // Block code: className is "language-xxx code-highlight"
    const langMatch = className.match(/language-(\S+)/)
    const lang = langMatch ? langMatch[1] : ''
    const isWrap = wrapLanguages.has(lang)
    return (
      <pre className={`text-sm rounded mb-3 ${isWrap ? 'bg-gray-900 text-gray-100 p-4 whitespace-pre-wrap break-words' : 'bg-gray-900 text-gray-100 p-4 overflow-x-auto'}`}>
        <code className={className} {...props}>{children}</code>
      </pre>
    )
  },
  pre: ({ children }) => <>{children}</>,
  blockquote: ({ children }) => (
    <blockquote className="border-l-4 border-gray-300 dark:border-gray-700 pl-4 italic text-gray-600 dark:text-gray-400 mb-3">{children}</blockquote>
  ),
  table: ({ children }) => (
    <div className="overflow-x-auto mb-3">
      <table className="min-w-full border-collapse border border-gray-300 dark:border-gray-700 text-sm">{children}</table>
    </div>
  ),
  th: ({ children }) => (
    <th className="border border-gray-300 dark:border-gray-700 bg-gray-100 dark:bg-gray-800 px-3 py-2 text-left font-semibold text-gray-700 dark:text-gray-200">{children}</th>
  ),
  td: ({ children }) => (
    <td className="border border-gray-300 dark:border-gray-700 px-3 py-2 text-gray-700 dark:text-gray-300">{children}</td>
  ),
  hr: () => <hr className="my-6 border-gray-200 dark:border-gray-700" />,
  strong: ({ children }) => <strong className="font-bold">{children}</strong>,
  em: ({ children }) => <em className="italic">{children}</em>,
}

export default function MarkdownRenderer({ content, className = '' }: Props) {
  const processed = preprocessLuoguMarkdown(content)
  return (
    <div className={`${className}`}>
      <ReactMarkdown
        remarkPlugins={[remarkGfm, remarkMath]}
        rehypePlugins={[rehypeRaw, [rehypePrism, { ignoreMissing: true }], rehypeKatex]}
        components={components}
      >
        {processed}
      </ReactMarkdown>
    </div>
  )
}
