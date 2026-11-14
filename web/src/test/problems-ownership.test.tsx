import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, within } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import ProblemsPage from "../features/problems/ProblemsPage";
import { ToastProvider } from "../errors/ToastContext";
import { useAuthStore } from "../stores/authStore";
import { getProblems } from "../services/problem.service";
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

const authoredProblem = {
  id: "p-mine",
  title: "我投稿的题目",
  // 用户改名后 author_name 可能滞后，归属判断必须依赖 author_id
  author_id: "u-2",
  author_name: "改名前的旧名字",
  contest: "",
  difficulty: "easy",
  status: "pending",
  created_at: "2026-01-01T00:00:00Z",
  public_at: null,
  claimed_by: null,
  has_verifier_solution: false,
};

const otherProblem = {
  ...authoredProblem,
  id: "p-other",
  title: "别人投稿的题目",
  author_id: "u-9",
  author_name: "改名后的新名字",
};

function setRenamedUser() {
  useAuthStore.setState({
    user: {
      id: "u-2",
      display_name: "改名后的新名字",
      role: "member",
      effective_role: "member",
      team_status: "joined",
    } as any,
    isAuthenticated: true,
    permMapLoading: false,
    permMap: {
      member: [
        "submit_problem",
        "view_pending_problems",
        "view_public_problems",
      ],
    } as any,
  });
}

describe("ProblemsPage 题目归属判断", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    (getProblems as ReturnType<typeof vi.fn>).mockResolvedValue([
      authoredProblem,
      otherProblem,
    ]);
    (getContests as ReturnType<typeof vi.fn>).mockResolvedValue([]);
    setRenamedUser();
  });

  it("修改显示名称后，「我的题目」仍按 author_id 命中自己的题目", async () => {
    render(
      <MemoryRouter>
        <ToastProvider>
          <ProblemsPage />
        </ToastProvider>
      </MemoryRouter>,
    );

    fireEvent.click(await screen.findByRole("button", { name: /我的题目/ }));

    expect(await screen.findByText("我投稿的题目")).toBeInTheDocument();
    expect(screen.queryByText("别人投稿的题目")).toBeNull();
  });

  it("「我的题目」徽标按 author_id 判断，不误标他人题目", async () => {
    render(
      <MemoryRouter>
        <ToastProvider>
          <ProblemsPage />
        </ToastProvider>
      </MemoryRouter>,
    );

    fireEvent.click(await screen.findByRole("button", { name: /全部题目/ }));

    const otherCard = (await screen.findByText("别人投稿的题目")).closest(
      "div.mg-box-shadow",
    );
    expect(otherCard).not.toBeNull();
    expect(within(otherCard as HTMLElement).queryByText("我的题目")).toBeNull();

    const mineCard = screen.getByText("我投稿的题目").closest("div.mg-box-shadow");
    expect(mineCard).not.toBeNull();
    expect(within(mineCard as HTMLElement).getByText("我的题目")).toBeInTheDocument();
  });
});
