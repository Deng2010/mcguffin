import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import ProblemsPage from "../features/problems/ProblemsPage";
import { ToastProvider } from "../errors/ToastContext";
import { useAuthStore } from "../stores/authStore";
import { getProblems, withdrawProblem } from "../services/problem.service";
import { getContests } from "../services/contest.service";

vi.mock("../services/problem.service", () => ({
  getProblems: vi.fn(),
  reviewProblem: vi.fn(),
  deleteProblem: vi.fn(),
  resubmitProblem: vi.fn(),
  withdrawProblem: vi.fn(),
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
  contest: "",
  difficulty: "easy",
  status: "pending",
  created_at: "2026-01-01T00:00:00Z",
  public_at: null,
  claimed_by: null,
  has_verifier_solution: false,
};

function setUser(id: string, role: string) {
  useAuthStore.setState({
    user: {
      id,
      display_name: id,
      role,
      effective_role: role,
      team_status: "joined",
    } as any,
    isAuthenticated: true,
    permMapLoading: false,
    permMap: {
      member: ["submit_problem", "view_pending_problems"],
      admin: ["approve_all_problems", "view_pending_problems"],
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

describe("ProblemsPage 出题人撤回待审核题目", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    (getProblems as ReturnType<typeof vi.fn>).mockResolvedValue([
      pendingProblem,
    ]);
    (getContests as ReturnType<typeof vi.fn>).mockResolvedValue([]);
    (withdrawProblem as ReturnType<typeof vi.fn>).mockResolvedValue({
      success: true,
      message: "已撤回",
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("出题人可在待审核列表撤回自己的题目", async () => {
    setUser("u-2", "member");
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(true);

    renderPage();
    fireEvent.click(await screen.findByRole("button", { name: /待审核/ }));

    fireEvent.click(await screen.findByRole("button", { name: "撤回" }));

    await waitFor(() =>
      expect(withdrawProblem).toHaveBeenCalledWith("p-1"),
    );
    expect(confirmSpy).toHaveBeenCalled();
  });

  it("取消确认时不会调用撤回接口", async () => {
    setUser("u-2", "member");
    vi.spyOn(window, "confirm").mockReturnValue(false);

    renderPage();
    fireEvent.click(await screen.findByRole("button", { name: /待审核/ }));
    fireEvent.click(await screen.findByRole("button", { name: "撤回" }));

    expect(withdrawProblem).not.toHaveBeenCalled();
  });

  it("管理员不能撤回他人的待审核题目（应使用「退回」）", async () => {
    setUser("admin", "admin");

    renderPage();
    fireEvent.click(await screen.findByRole("button", { name: /待审核/ }));

    await screen.findByText("待审核的题目");
    expect(screen.queryByRole("button", { name: "撤回" })).toBeNull();
    expect(screen.getByRole("button", { name: "退回" })).toBeInTheDocument();
  });
});
