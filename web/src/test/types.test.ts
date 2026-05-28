import { describe, it, expect } from 'vitest'
import { defaultRolePermissions, type Permission } from '../types'

describe('defaultRolePermissions', () => {
  it('superadmin gets all permissions', () => {
    const perms = defaultRolePermissions.superadmin
    // Superadmin has wildcard on backend — frontend fallback should be complete
    expect(perms.length).toBeGreaterThanOrEqual(17)
    const required: Permission[] = [
      'view_showcase', 'view_team', 'manage_team', 'manage_members', 'submit_problem',
      'view_problems', 'approve_problem', 'manage_contests', 'manage_site',
      'view_discussions', 'manage_discussions', 'manage_tags',
      'manage_notifications', 'manage_backups', 'view_stats', 'manage_posts',
    ]
    required.forEach(p => { expect(perms).toContain(p) })
  })

  it('admin has all admin permissions (minus backup)', () => {
    const perms = defaultRolePermissions.admin
    expect(perms).toContain('manage_site')
    expect(perms).toContain('manage_members')
    expect(perms).toContain('approve_problem')
    // Backups are superadmin-only in defaults
    expect(perms).not.toContain('manage_backups')
  })

  it('member has limited permissions', () => {
    const perms = defaultRolePermissions.member
    expect(perms).toContain('view_showcase')
    expect(perms).toContain('view_team')
    expect(perms).toContain('submit_problem')
    expect(perms).toContain('view_problems')
    // Member should NOT have admin permissions
    expect(perms).not.toContain('approve_problem')
    expect(perms).not.toContain('manage_team')
    expect(perms).not.toContain('manage_contests')
    expect(perms).not.toContain('manage_site')
  })

  it('guest has minimal permissions', () => {
    const perms = defaultRolePermissions.guest
    expect(perms).toContain('view_showcase')
    expect(perms).toContain('apply_join')
    expect(perms).not.toContain('view_problems')
    expect(perms).not.toContain('submit_problem')
    expect(perms).not.toContain('view_team')
  })

  it('every role has view_showcase', () => {
    for (const perms of Object.values(defaultRolePermissions)) {
      expect(perms).toContain('view_showcase')
    }
  })
})
