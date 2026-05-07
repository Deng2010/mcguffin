import { useState } from 'react'
import { Link } from 'react-router-dom'

export default function NotFoundPage() {
  const [code, setCode] = useState('')
  const [submitted, setSubmitted] = useState(false)

  const handleSubmit = () => {
    setSubmitted(true)
    setTimeout(() => {
      window.location.hash = '#/'
    }, 800)
  }

  return (
    <div className="p-6 max-w-4xl mx-auto">
      <Link to="/" className="text-sm text-gray-500 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-100 mb-4 inline-block">
        ← 返回首页
      </Link>

      <div className="bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 p-6">
        {/* Problem header */}
        <div className="flex items-start justify-between mb-4">
          <div>
            <h1 className="text-2xl font-bold text-gray-800 dark:text-gray-100">
              P404 Not Found
            </h1>
            <div className="flex flex-wrap gap-x-4 gap-y-1 text-sm text-gray-500 dark:text-gray-400 mt-2">
              <span>时间限制：1.00s</span>
              <span>内存限制：256.00MB</span>
              <span>难度：Unknown</span>
            </div>
          </div>
        </div>

        {/* Problem description */}
        <div className="mt-6">
          <h2 className="text-base font-semibold mb-3 text-gray-700 dark:text-gray-200 border-l-4 border-gray-800 dark:border-gray-200 pl-3">
            题目描述
          </h2>
          <div className="text-sm text-gray-600 dark:text-gray-300 leading-relaxed space-y-2">
            <p>
              你试图访问的页面不在本题空间中。
            </p>
            <p>
              给定一个错误的请求 URL，请判断是否存在从当前 URL 到有效页面的路径。
            </p>
            <p>
              由于这是一道不可解题，你只需要输出一行 "404" 即可获得满分。
            </p>
          </div>
        </div>

        {/* Input format */}
        <div className="mt-6">
          <h2 className="text-base font-semibold mb-3 text-gray-700 dark:text-gray-200 border-l-4 border-gray-800 dark:border-gray-200 pl-3">
            输入格式
          </h2>
          <div className="text-sm text-gray-600 dark:text-gray-300 leading-relaxed">
            <p>本题没有输入。</p>
          </div>
        </div>

        {/* Output format */}
        <div className="mt-6">
          <h2 className="text-base font-semibold mb-3 text-gray-700 dark:text-gray-200 border-l-4 border-gray-800 dark:border-gray-200 pl-3">
            输出格式
          </h2>
          <div className="text-sm text-gray-600 dark:text-gray-300 leading-relaxed">
            <p>一行，包含字符串 <code className="px-1.5 py-0.5 bg-gray-100 dark:bg-gray-800 text-gray-800 dark:text-gray-200 font-mono">404</code>。</p>
          </div>
        </div>

        {/* Sample */}
        <div className="mt-6">
          <h2 className="text-base font-semibold mb-3 text-gray-700 dark:text-gray-200 border-l-4 border-gray-800 dark:border-gray-200 pl-3">
            样例
          </h2>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <div>
              <div className="text-xs font-semibold text-gray-500 dark:text-gray-400 mb-1">输入</div>
              <pre className="px-4 py-3 bg-gray-50 dark:bg-gray-800 border border-gray-200 dark:border-gray-700 text-sm font-mono text-gray-700 dark:text-gray-300 overflow-x-auto">（空）</pre>
            </div>
            <div>
              <div className="text-xs font-semibold text-gray-500 dark:text-gray-400 mb-1">输出</div>
              <pre className="px-4 py-3 bg-gray-50 dark:bg-gray-800 border border-gray-200 dark:border-gray-700 text-sm font-mono text-gray-700 dark:text-gray-300 overflow-x-auto">404</pre>
            </div>
          </div>
        </div>

        {/* Code editor area */}
        <div className="mt-8">
          <h2 className="text-base font-semibold mb-3 text-gray-700 dark:text-gray-200 border-l-4 border-gray-800 dark:border-gray-200 pl-3">
            代码
          </h2>
          <div className="border border-gray-300 dark:border-gray-700">
            {/* Editor title bar */}
            <div className="flex items-center gap-1.5 px-3 py-2 bg-gray-100 dark:bg-gray-800 border-b border-gray-300 dark:border-gray-700">
              <span className="w-2.5 h-2.5 inline-block bg-red-400" />
              <span className="w-2.5 h-2.5 inline-block bg-yellow-400" />
              <span className="w-2.5 h-2.5 inline-block bg-green-400" />
              <span className="text-xs text-gray-400 dark:text-gray-500 ml-auto font-mono">main.cpp</span>
            </div>
            {/* Editor body */}
            <textarea
              value={code}
              onChange={e => setCode(e.target.value)}
              rows={8}
              className="w-full px-4 py-3 border-0 bg-gray-50 dark:bg-gray-950 focus:outline-none font-mono text-sm text-gray-700 dark:text-gray-300 resize-y"
              placeholder={'#include <iostream>\nusing namespace std;\n\nint main() {\n  cout << "404" << endl;\n  return 0;\n}'}
              disabled={submitted}
            />
          </div>
        </div>

        {/* Submit button */}
        <div className="mt-4 flex items-center justify-between">
          <div className="text-xs text-gray-400 dark:text-gray-500 font-mono">
            {submitted ? '已提交，正在评测...' : '语言：C++17'}
          </div>
          <button
            onClick={handleSubmit}
            disabled={submitted}
            className={`px-6 py-2 text-sm border select-none ${
              submitted
                ? 'bg-green-50 dark:bg-green-900/30 border-green-300 dark:border-green-800 text-green-600 dark:text-green-400'
                : 'bg-gray-800 dark:bg-gray-700 text-white border-gray-900 dark:border-gray-600 hover:bg-gray-700 dark:hover:bg-gray-600'
            }`}
          >
            {submitted ? 'Accepted!' : '提交评测'}
          </button>
        </div>

        {/* Judge result */}
        {submitted && (
          <div className="mt-4 p-4 bg-green-50 dark:bg-green-900/30 border border-green-300 dark:border-green-800">
            <div className="flex items-center gap-2">
              <span className="text-lg font-bold text-green-600 dark:text-green-400">Accepted</span>
              <span className="text-xs text-green-500 dark:text-green-500">100pts</span>
            </div>
            <div className="text-xs text-green-600 dark:text-green-400 mt-1">
              正在将您重定向至首页...
            </div>
          </div>
        )}

        {/* Footer hint */}
        <div className="mt-6 pt-4 border-t border-gray-200 dark:border-gray-800 text-center text-xs text-gray-400 dark:text-gray-500">
          如果你确信这是一个有效路径，请检查 URL 是否拼写正确，或联系管理员添加该页面。
        </div>
      </div>
    </div>
  )
}
