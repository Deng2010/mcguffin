/** 是否已加入团队（team_status === "joined"） */
export function isJoined(
  user: { team_status: string } | null | undefined,
): boolean {
  return user?.team_status === "joined";
}
