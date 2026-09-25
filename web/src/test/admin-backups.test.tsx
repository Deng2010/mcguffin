import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import AdminBackupsPage from "../features/admin/AdminBackupsPage";
import { ToastProvider } from "../errors/ToastContext";
import {
  deleteBackup,
  downloadBackup,
  getBackups,
} from "../services/admin.service";

vi.mock("../services/admin.service", () => ({
  getBackups: vi.fn(),
  createBackup: vi.fn(),
  deleteBackup: vi.fn(),
  downloadBackup: vi.fn(),
  restoreBackup: vi.fn(),
  restoreFromUpload: vi.fn(),
  importData: vi.fn(),
  importConfig: vi.fn(),
  exportData: vi.fn(),
  exportDatabase: vi.fn(),
}));

const backups = [
  { name: "mcguffin_2026_a.db", size: 2048, modified: "2026-01-01 10:00:00" },
  { name: "mcguffin_2026_b.db", size: 4096, modified: "2026-01-02 10:00:00" },
  { name: "pre_restore_c.db", size: 8192, modified: "2026-01-03 10:00:00" },
];

const getBackupsMock = vi.mocked(getBackups);
const deleteBackupMock = vi.mocked(deleteBackup);
const downloadBackupMock = vi.mocked(downloadBackup);

function renderPage() {
  return render(
    <ToastProvider>
      <AdminBackupsPage />
    </ToastProvider>,
  );
}

const rowCheckbox = (name: string) =>
  screen.getByLabelText(`选择备份 ${name}`) as HTMLInputElement;
const selectAll = () => screen.getByLabelText("全选备份") as HTMLInputElement;

beforeEach(() => {
  vi.clearAllMocks();
  getBackupsMock.mockResolvedValue({
    success: true,
    backups,
  } as any);
  deleteBackupMock.mockResolvedValue({ success: true } as any);
  downloadBackupMock.mockResolvedValue({
    success: true,
    filename: "mcguffin_2026_a.db",
    mime: "application/x-sqlite3",
    encoding: "base64",
    content: "AAA=",
  } as any);
  URL.createObjectURL = vi.fn(() => "blob:mock");
  URL.revokeObjectURL = vi.fn();
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("AdminBackupsPage 多选/全选", () => {
  it("全选后所有备份被勾选，并可取消选择", async () => {
    renderPage();
    await screen.findByLabelText("选择备份 mcguffin_2026_a.db");

    expect(selectAll()).not.toBeChecked();
    fireEvent.click(selectAll());

    for (const b of backups) {
      expect(rowCheckbox(b.name)).toBeChecked();
    }
    expect(selectAll()).toBeChecked();
    expect(selectAll().indeterminate).toBe(false);
    expect(screen.getByText("已选 3 / 3 项")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "取消选择" }));
    for (const b of backups) {
      expect(rowCheckbox(b.name)).not.toBeChecked();
    }
    expect(screen.queryByRole("button", { name: "批量删除" })).toBeNull();
  });

  it("部分勾选时全选框为半选状态，再次点击全选可清空", async () => {
    renderPage();
    await screen.findByLabelText("选择备份 mcguffin_2026_a.db");

    fireEvent.click(rowCheckbox("mcguffin_2026_b.db"));
    expect(screen.getByText("已选 1 / 3 项")).toBeInTheDocument();
    expect(selectAll()).not.toBeChecked();
    expect(selectAll().indeterminate).toBe(true);

    // 半选状态下点击全选 → 选中全部
    fireEvent.click(selectAll());
    expect(rowCheckbox("mcguffin_2026_a.db")).toBeChecked();
    expect(screen.getByText("已选 3 / 3 项")).toBeInTheDocument();

    // 已全选时再次点击 → 清空
    fireEvent.click(selectAll());
    expect(screen.queryByText("已选 3 / 3 项")).toBeNull();
    expect(selectAll()).not.toBeChecked();
  });

  it("批量删除逐条调用删除接口、刷新列表并清空选择", async () => {
    vi.spyOn(window, "confirm").mockReturnValue(true);
    getBackupsMock
      .mockResolvedValueOnce({ success: true, backups } as any)
      .mockResolvedValueOnce({ success: true, backups: [] } as any);

    renderPage();
    await screen.findByLabelText("选择备份 mcguffin_2026_a.db");

    fireEvent.click(selectAll());
    fireEvent.click(screen.getByRole("button", { name: "批量删除" }));

    await waitFor(() => expect(deleteBackupMock).toHaveBeenCalledTimes(3));
    expect(deleteBackupMock.mock.calls.map((c) => c[0])).toEqual(
      backups.map((b) => b.name),
    );

    expect(await screen.findByText("暂无备份")).toBeInTheDocument();
    expect(getBackupsMock).toHaveBeenCalledTimes(2);
    expect(screen.queryByRole("button", { name: "批量删除" })).toBeNull();
  });

  it("批量下载依次下载选中项", async () => {
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(true);
    renderPage();
    await screen.findByLabelText("选择备份 mcguffin_2026_a.db");

    fireEvent.click(rowCheckbox("mcguffin_2026_a.db"));
    fireEvent.click(rowCheckbox("pre_restore_c.db"));
    fireEvent.click(screen.getByRole("button", { name: "批量下载" }));

    await waitFor(() => expect(downloadBackupMock).toHaveBeenCalledTimes(2), {
      timeout: 3000,
    });
    expect(downloadBackupMock.mock.calls.map((c) => c[0])).toEqual([
      "mcguffin_2026_a.db",
      "pre_restore_c.db",
    ]);
    expect(confirmSpy).toHaveBeenCalled();
    expect(URL.createObjectURL).toHaveBeenCalledTimes(2);
    expect(deleteBackupMock).not.toHaveBeenCalled();
  });

  it("批量删除存在失败项时仍刷新列表并清空选择", async () => {
    vi.spyOn(window, "confirm").mockReturnValue(true);
    deleteBackupMock.mockImplementation((name: string) =>
      Promise.resolve(
        name === "mcguffin_2026_b.db"
          ? ({ success: false, message: "备份文件不存在" } as any)
          : ({ success: true } as any),
      ),
    );

    renderPage();
    await screen.findByLabelText("选择备份 mcguffin_2026_a.db");

    fireEvent.click(selectAll());
    fireEvent.click(screen.getByRole("button", { name: "批量删除" }));

    await waitFor(() => expect(deleteBackupMock).toHaveBeenCalledTimes(3));
    expect(getBackupsMock).toHaveBeenCalledTimes(2);
    expect(screen.queryByRole("button", { name: "批量删除" })).toBeNull();
  });
});
