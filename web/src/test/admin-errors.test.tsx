import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import AdminErrorsPage from "../features/admin/AdminErrorsPage";
import { ToastProvider } from "../errors/ToastContext";
import {
  fetchErrorReports,
  updateErrorStatus,
} from "../services/error.service";

vi.mock("../services/error.service", () => ({
  fetchErrorReports: vi.fn(),
  updateErrorStatus: vi.fn(),
  deleteError: vi.fn(),
  clearErrors: vi.fn(),
}));

const row = {
  id: "err-1",
  ts: "2026-08-04T10:00:00Z",
  user_id: "admin",
  source: "frontend",
  code: "PROBLEM_NOT_FOUND",
  message: "题目不存在",
  hint: "检查题目 ID",
  suggestion: "题目不存在或已被删除",
  stack: "at handleClick (file.ts:1:1)",
  url: "http://localhost/#/problems",
  route: "#/problems",
  method: "GET",
  http_status: 404,
  ua: "vitest",
  plugin_id: "",
  count: 3,
  status: "open",
  resolved_by: null,
  resolved_at: null,
  first_seen: "2026-08-04T10:00:00Z",
  last_seen: "2026-08-04T10:10:00Z",
};

describe("AdminErrorsPage 错误中心", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    (fetchErrorReports as ReturnType<typeof vi.fn>).mockResolvedValue({
      success: true,
      errors: [row],
      total: 1,
    });
    (updateErrorStatus as ReturnType<typeof vi.fn>).mockResolvedValue({
      success: true,
      message: "已更新",
    });
  });

  it("渲染错误列表（错误码、消息、次数、状态）", async () => {
    render(
      <ToastProvider>
        <AdminErrorsPage />
      </ToastProvider>,
    );
    await waitFor(() => {
      expect(screen.getByText("PROBLEM_NOT_FOUND")).toBeInTheDocument();
    });
    expect(screen.getByText("题目不存在")).toBeInTheDocument();
    expect(screen.getByText("3")).toBeInTheDocument();
    expect(screen.getAllByText("待处理").length).toBeGreaterThan(0);
  });

  it("点击行打开详情并展示修改建议，状态流转调用接口", async () => {
    render(
      <ToastProvider>
        <AdminErrorsPage />
      </ToastProvider>,
    );
    await waitFor(() => {
      fireEvent.click(screen.getByText("PROBLEM_NOT_FOUND"));
    });
    expect(screen.getByText("错误详情")).toBeInTheDocument();
    expect(screen.getByText("题目不存在或已被删除")).toBeInTheDocument();
    const buttons = screen.getAllByText("已解决");
    fireEvent.click(buttons[buttons.length - 1]);
    expect(updateErrorStatus).toHaveBeenCalledWith("err-1", "resolved");
  });
});
