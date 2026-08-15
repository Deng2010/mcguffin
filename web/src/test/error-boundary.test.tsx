import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import ErrorBoundary from "../errors/ErrorBoundary";
import { reportNormalizedError } from "../errors/reporter";

vi.mock("../errors/reporter", () => ({
  reportError: vi.fn(),
  initErrorCapture: vi.fn(),
  reportNormalizedError: vi.fn(),
}));

function Bomb(): never {
  throw new Error("render boom");
}

describe("ErrorBoundary", () => {
  it("捕获渲染错误、自动上报并展示降级页", () => {
    render(
      <ErrorBoundary>
        <Bomb />
      </ErrorBoundary>,
    );
    expect(screen.getByText("页面出错了")).toBeInTheDocument();
    expect(screen.getByText("render boom")).toBeInTheDocument();
    expect(screen.getByText("重新加载")).toBeInTheDocument();
    expect(reportNormalizedError).toHaveBeenCalled();
  });

  it("正常子组件不触发降级", () => {
    render(
      <ErrorBoundary>
        <div>正常内容</div>
      </ErrorBoundary>,
    );
    expect(screen.getByText("正常内容")).toBeInTheDocument();
    expect(screen.queryByText("页面出错了")).not.toBeInTheDocument();
  });
});
