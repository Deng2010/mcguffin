import { describe, it, expect } from "vitest";
import { render } from "@testing-library/react";
import { existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import MarkdownRenderer, {
  preprocessTexMath,
} from "../components/MarkdownRenderer";

/** 渲染 Markdown 并返回容器。 */
function renderMd(md: string) {
  return render(<MarkdownRenderer content={md} />).container;
}

/** 公式之外的文本（KaTeX 的 MathML 里会保留 LaTeX 源码，需要排除）。 */
function textOutsideMath(container: HTMLElement): string {
  const clone = container.cloneNode(true) as HTMLElement;
  clone.querySelectorAll(".katex").forEach((el) => el.remove());
  return clone.textContent ?? "";
}

describe("MarkdownRenderer — TeX 分隔符 \\( \\) / \\[ \\]", () => {
  it("渲染 \\( ... \\) 行内公式", () => {
    const container = renderMd("设 \\(a \\neq b\\) 成立");
    expect(container.querySelectorAll(".katex").length).toBe(1);
    // 不应该再留下原始的 LaTeX 源码
    const text = textOutsideMath(container);
    expect(text).toContain("设");
    expect(text).toContain("成立");
    expect(text).not.toContain("\\neq");
    expect(text).not.toContain("\\(");
  });

  it("渲染 \\[ ... \\] 块级公式（display 模式）", () => {
    const container = renderMd("推导：\n\\[a \\neq b\\]\n结束");
    expect(container.querySelectorAll(".katex").length).toBe(1);
    expect(container.querySelectorAll(".katex-display").length).toBe(1);
    expect(textOutsideMath(container)).not.toContain("\\[");
  });

  it("行内公式里的换行会被折叠（\\(...\\) 可跨行书写）", () => {
    const container = renderMd("\\(a\n+ b\\)");
    expect(container.querySelectorAll(".katex").length).toBe(1);
  });

  it("公式内容里的 $ 不会提前结束公式", () => {
    const container = renderMd("\\(a $ b\\)");
    expect(container.querySelectorAll(".katex").length).toBe(1);
    expect(container.querySelectorAll(".katex-error").length).toBe(0);
  });

  it("代码块与行内代码里的 LaTeX 原样保留", () => {
    const container = renderMd(
      "```\n\\(a \\neq b\\)\n\\[c\\]\n```\n\n`\\(d\\)`",
    );
    expect(container.querySelectorAll(".katex").length).toBe(0);
    expect(container.textContent).toContain("\\(a \\neq b\\)");
    expect(container.textContent).toContain("\\[c\\]");
    expect(container.textContent).toContain("\\(d\\)");
  });

  it("$ / $$ 公式不受影响", () => {
    const container = renderMd("行内 $a \\neq b$ 与\n\n$$\nc + d\n$$");
    expect(container.querySelectorAll(".katex").length).toBe(2);
    expect(container.querySelectorAll(".katex-display").length).toBe(1);
  });

  it("\\neq 渲染为非等于号（不是斜杠叠等号）", () => {
    const container = renderMd("$a \\neq b$");
    // MathML 分支是真正的 ≠ 字符
    expect(container.querySelector(".katex-mathml")!.textContent).toContain(
      "≠",
    );
    // HTML 分支把斜杠叠加在 = 上，依赖 katex CSS 的 `.katex .rlap>.inner` 定位
    const html = container.querySelector(".katex-html")!.innerHTML;
    expect(html).toContain('class="rlap"');
    expect(html).toContain('class="inner"');
    expect(html).toContain("=");
  });

  it("preprocessTexMath 处理相邻公式并跳过转义反斜杠", () => {
    expect(preprocessTexMath("\\(a\\) 与 \\(b\\)")).toBe("$a$ 与 $b$");
    expect(preprocessTexMath("\\[a\\]")).toBe("\n$$\na\n$$\n");
    // 已经被转义的反斜杠不当作分隔符
    expect(preprocessTexMath("\\\\(a\\\\)")).toBe("\\\\(a\\\\)");
  });
});

describe("KaTeX CSS 与渲染器版本一致", () => {
  // 测试通常在 `web/` 下运行，也支持从仓库根运行，因此向上找依赖目录。
  const KATEX_CSS = "web/node_modules/katex/dist/katex.min.css";

  function findKatexCss(): string {
    for (const candidate of [".", "..", "../.."]) {
      const path = join(resolve(candidate), KATEX_CSS);
      if (existsSync(path)) return path;
    }
    throw new Error(
      `找不到 katex 的 CSS（${KATEX_CSS}）：请先在 web/ 安装依赖`,
    );
  }

  /**
   * katex 0.18 起把结构类名加了 `katex-` 前缀（`.base` → `.katex-base`、
   * `.rlap > .inner` → `.rlap > .katex-inner` …），而 rehype-katex@7 用
   * katex 0.16 生成 **无前缀** 类名。若应用里 `katex`（只用来引 CSS）与
   * rehype-katex 依赖的版本不一致，结构类就没有样式，`\neq` 的斜杠不再叠加
   * 在 `=` 上（看起来像 "/="），因此这里把「CSS 里存在渲染器实际输出的类名」
   * 固定成回归测试。
   */
  it("katex.min.css 定义了渲染输出用到的结构类", () => {
    const css = readFileSync(findKatexCss(), "utf8");
    const container = renderMd("$a \\neq b$");
    const html = container.querySelector(".katex-html")!.innerHTML;

    // katex 0.18 把这些选择器改成了 `.katex-base` / `.rlap>.katex-inner` …；
    // 渲染输出仍是无前缀类名时，这些规则必须能在 CSS 里找到。
    const requiredSelectors = [
      ".katex .base",
      ".katex .strut",
      ".katex .vbox",
      ".katex .thinbox",
      ".katex .rlap>.inner",
      ".katex .rlap>.fix",
    ];
    const usedClasses = [
      "base",
      "strut",
      "vbox",
      "thinbox",
      "rlap",
      "inner",
      "fix",
    ];

    // 前提：渲染输出确实还在用无前缀类名
    for (const cls of usedClasses) {
      expect(
        new RegExp(`class="[^"]*\\b${cls}\\b`).test(html),
        `渲染输出里没有 .${cls}，katex 输出格式可能已变`,
      ).toBe(true);
    }

    const missing = requiredSelectors.filter((sel) => !css.includes(sel));
    expect(
      missing,
      `katex 版本不匹配：CSS 里缺少 ${missing.join(", ")}。` +
        `请让 katex（CSS）与 rehype-katex 依赖的版本保持一致（rehype-katex@7 → katex 0.16.x）。`,
    ).toEqual([]);
  });
});
