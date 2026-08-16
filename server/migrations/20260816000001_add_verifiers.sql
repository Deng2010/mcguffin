-- 多验题人支持：新增 verifiers 列（JSON 数组，每个元素为验题人条目）
-- 每条 VerifierEntry: { user_id, user_name, solution, comments: [...], claimed_at }
-- 保留 claimed_by / verifier_solution 作为“主验题人”的向后兼容镜像。
ALTER TABLE problems ADD COLUMN verifiers TEXT NOT NULL DEFAULT '[]';
