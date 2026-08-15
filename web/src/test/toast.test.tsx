import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent, act } from "@testing-library/react";
import { ToastProvider, useToast } from "../errors/ToastContext";

function Trigger() {
  const toast = useToast();
  return (
    <div>
      <button onClick={() => toast.error("出错了", "错误详情")}>触发错误</button>
      <button onClick={() => toast.success("成功了")}>触发成功</button>
    </div>
  );
}

describe("ToastProvider", () => {
  it("展示错误 toast 并可展开详情", () => {
    render(
      <ToastProvider>
        <Trigger />
      </ToastProvider>,
    );
    fireEvent.click(screen.getByText("触发错误"));
    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.getByText("出错了")).toBeInTheDocument();
    fireEvent.click(screen.getByText("查看详情"));
    expect(screen.getByText("错误详情")).toBeInTheDocument();
  });

  it("展示成功 toast", () => {
    render(
      <ToastProvider>
        <Trigger />
      </ToastProvider>,
    );
    fireEvent.click(screen.getByText("触发成功"));
    expect(screen.getByText("成功了")).toBeInTheDocument();
  });

  it("自动消失（错误 8 秒）", async () => {
    vi.useFakeTimers();
    render(
      <ToastProvider>
        <Trigger />
      </ToastProvider>,
    );
    fireEvent.click(screen.getByText("触发错误"));
    expect(screen.getByRole("alert")).toBeInTheDocument();
    act(() => {
      vi.advanceTimersByTime(8500);
    });
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    vi.useRealTimers();
  });
});
