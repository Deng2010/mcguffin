import { describe, it, expect } from 'vitest'
import { rolePermissions, type User, type Permission } from '../types'

describe('rolePermissions', () => {
  it('superadmin has all permissions', () => {
    const permissions = rolePermissions.superadmin
    const allPermissions: Permission[] = [
      'view_showcase', 'view_team', 'manage_team', 'submit_problem',
      'view_problems', 'approve_problem', 'edit_contests', 'manage_site',
    ]
    allPermissions.forEach(p => {
      expect(permissions).toContain(p)
    })
    expect(permissions.length).toBe(allPermissions.length)
  })

  it('admin has same permissions as superadmin', () => {
    expect(rolePermissions.admin).toEqual(rolePermissions.superadmin)
  })

  it('member has limited permissions', () => {
    const permissions = rolePermissions.member
    expect(permissions).toContain('view_showcase')
    expect(permissions).toContain('view_team')
    expect(permissions).toContain('submit_problem')
    expect(permissions).toContain('view_problems')
    // Member should NOT have admin permissions
    expect(permissions).not.toContain('approve_problem')
    expect(permissions).not.toContain('manage_team')
    expect(permissions).not.toContain('edit_contests')
    expect(permissions).not.toContain('manage_site')
  })

  it('guest has minimal permissions', () => {
    const permissions = rolePermissions.guest
    expect(permissions).toContain('view_showcase')
    expect(permissions).toContain('apply_join')
    expect(permissions).not.toContain('view_problems')
    expect(permissions).not.toContain('submit_problem')
    expect(permissions).not.toContain('view_team')
  })

  it('pending user has same permissions as guest', () => {
    expect(rolePermissions.pending).toEqual(rolePermissions.guest)
  })

  it('every role has view_showcase', () => {
    const roles: User['role'][] = ['superadmin', 'admin', 'member', 'guest', 'pending']
    roles.forEach(role => {
      expect(rolePermissions[role]).toContain('view_showcase')
    })
  })
})
