import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import ProblemsPage from "../features/problems/ProblemsPage";
import { ToastProvider } from "../errors/ToastContext";
import { useAuthStore } from "../stores/authStore";
import { getProblems, reviewProblem } from "../services/problem.service";
import { getContests } from "../services/contest.service";

vi.mock("../services/problem.service", () => ({
  getProblems: vi.fn(),
  reviewProblem: vi.fn(),
  deleteProblem: vi.fn(),
  resubmitProblem: vi.fn(),
  claimProblem: vi.fn(),
  unclaimProblem: vi.fn(),
  setProblemContest: vi.fn(),
  setProblemVisibility: vi.fn(),
}));

vi.mock("../services/contest.service", () => ({
  getContests: vi.fn(),
}));

const pendingProblem = {
  id: "p-1",
  title: "待审核的题目",
  author_id: "u-2",
  author_name: "出题人",
  contest: "测试赛",
  difficulty: "easy",
  status: "pending",
  created_at: "2026-01-01T00:00:00Z",
  public_at: null,
  claimed_by: null,
  has_verifier_solution: false,
};

function setAdmin() {
  useAuthStore.setState({
    user: {
      id: "admin",
      display_name: "管理员",
      role: "admin",
      effective_role: "admin",
      team_status: "joined",
    } as any,
    isAuthenticated: true,
    permMapLoading: false,
    permMap: {
      admin: [
        "approve_all_problems",
        "submit_problem",
        "view_pending_problems",
        "view_approved_problems",
        "view_public_problems",
      ],
    } as any,
  });
}

function renderPage() {
  return render(
    <MemoryRouter>
      <ToastProvider>
        <ProblemsPage />
      </ToastProvider>
    </MemoryRouter>,
  );
}

describe("ProblemsPage 待审核 Tab", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    (getProblems as ReturnType<typeof vi.fn>).mockResolvedValue([
      pendingProblem,
    ]);
    (getContests as ReturnType<typeof vi.fn>).mockResolvedValue([]);
    (reviewProblem as ReturnType<typeof vi.fn>).mockResolvedValue({
      success: true,
      message: "",
    });
    setAdmin();
  });

  it("渲染与其它 Tab 一致的题目卡片，并在右上角提供通过/回复/退回", async () => {
    renderPage();
    fireEvent.click(await screen.findByRole("button", { name: /待审核/ }));

    // 卡片正文：标题 + 作者/赛事/难度/状态等元信息
    expect(await screen.findByText("待审核的题目")).toBeInTheDocument();
    expect(screen.getByText(/作者：出题人/)).toBeInTheDocument();
    expect(screen.getByText(/赛事：测试赛/)).toBeInTheDocument();

    // 右上角操作按钮
    expect(screen.getByRole("button", { name: "通过" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "回复" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "退回" })).toBeInTheDocument();

    // 卡片上不应再出现可见性设置
    expect(screen.queryByText(/可见性设置/)).toBeNull();
    expect(screen.queryByText("保存可见性")).toBeNull();
  });

  it("点击通过调用 approve 审核动作", async () => {
    renderPage();
    fireEvent.click(await screen.findByRole("button", { name: /待审核/ }));
    fireEvent.click(await screen.findByRole("button", { name: "通过" }));

    await waitFor(() =>
      expect(reviewProblem).toHaveBeenCalledWith("p-1", "approve", undefined),
    );
  });

  it("点击退回打开退回理由弹窗（reject）", async () => {
    renderPage();
    fireEvent.click(await screen.findByRole("button", { name: /待审核/ }));
    fireEvent.click(await screen.findByRole("button", { name: "退回" }));

    expect(await screen.findByText("退回理由")).toBeInTheDocument();
    expect(screen.getByText(/不少于 10 个字/)).toBeInTheDocument();
  });
});
