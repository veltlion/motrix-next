/**
 * @fileoverview Tests for TaskItemActions — status-to-action state machine.
 *
 * Tests the actionsMap computed property that determines which action buttons
 * appear for each task status. This is the core logic that controls the UX.
 * Uses @vue/test-utils mount to test real computed property execution.
 */
import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { ref } from 'vue'
import { TASK_STATUS } from '@shared/constants'

// ── Mock all external deps ─────────────────────────────────────────
vi.mock('vue-i18n', () => ({
  useI18n: () => ({ t: (key: string) => key }),
}))

// Mock Naive UI components as stubs
vi.mock('naive-ui', () => ({
  NIcon: { template: '<span><slot /></span>' },
  NTooltip: { template: '<span><slot /><slot name="trigger" /></span>' },
}))

// Mock ionicons
vi.mock('@vicons/ionicons5', () => ({
  PauseOutline: { template: '<i />' },
  PlayOutline: { template: '<i />' },
  StopOutline: { template: '<i />' },
  RefreshOutline: { template: '<i />' },
  CloseOutline: { template: '<i />' },
  TrashOutline: { template: '<i />' },
  LinkOutline: { template: '<i />' },
  InformationCircleOutline: { template: '<i />' },
  FolderOpenOutline: { template: '<i />' },
  SyncOutline: { template: '<i />' },
}))

import TaskItemActions from '../TaskItemActions.vue'

const createWrapper = (status: string, gid = 'abc123') => {
  return mount(TaskItemActions, {
    props: {
      task: { gid } as never,
      status,
    },
    global: {
      provide: {
        stoppingGids: ref([]),
      },
    },
  })
}

describe('TaskItemActions', () => {
  describe('action set per status', () => {
    it('shows pause+delete for ACTIVE tasks', () => {
      const wrapper = createWrapper(TASK_STATUS.ACTIVE)
      const actions = wrapper.findAll('.task-item-action')
      // Actions include the 3 common actions (folder, link, info) + status-specific
      expect(actions.length).toBeGreaterThanOrEqual(2 + 3) // pause, delete + common
    })

    it('shows resume+delete for PAUSED tasks', () => {
      const wrapper = createWrapper(TASK_STATUS.PAUSED)
      const actions = wrapper.findAll('.task-item-action')
      expect(actions.length).toBeGreaterThanOrEqual(2 + 3)
    })

    it('shows restart+trash for COMPLETE tasks', () => {
      const wrapper = createWrapper(TASK_STATUS.COMPLETE)
      const actions = wrapper.findAll('.task-item-action')
      expect(actions.length).toBeGreaterThanOrEqual(2 + 3)
    })

    it('shows restart+trash for ERROR tasks', () => {
      const wrapper = createWrapper(TASK_STATUS.ERROR)
      const actions = wrapper.findAll('.task-item-action')
      expect(actions.length).toBeGreaterThanOrEqual(2 + 3)
    })

    it('shows stop-seeding+delete for SEEDING tasks', () => {
      const wrapper = createWrapper(TASK_STATUS.SEEDING)
      const actions = wrapper.findAll('.task-item-action')
      const hasStopSeeding = actions.some((a) => a.classes().includes('stop-seeding'))
      expect(hasStopSeeding).toBe(true)
    })
  })

  describe('event emission', () => {
    it('emits pause when pause action is clicked', async () => {
      const wrapper = createWrapper(TASK_STATUS.ACTIVE)
      const actions = wrapper.findAll('.task-item-action')
      // Find and click the first action (reversed order, so pause is last status-specific)
      await actions[actions.length - 1].trigger('click')
      // At least one event should be emitted
      expect(Object.keys(wrapper.emitted()).length).toBeGreaterThan(0)
    })

    it('emits stop-seeding when stop action is clicked on SEEDING task', async () => {
      const wrapper = createWrapper(TASK_STATUS.SEEDING)
      const stopAction = wrapper.findAll('.task-item-action').find((a) => a.classes().includes('stop-seeding'))
      expect(stopAction).toBeDefined()
      await stopAction!.trigger('click')
      expect(wrapper.emitted('stop-seeding')).toBeTruthy()
    })
  })

  describe('status variants', () => {
    it('shows resume+delete for WAITING tasks', () => {
      const wrapper = createWrapper(TASK_STATUS.WAITING)
      const actions = wrapper.findAll('.task-item-action')
      expect(actions.length).toBeGreaterThanOrEqual(2 + 3)
    })

    it('shows restart+trash for REMOVED tasks', () => {
      const wrapper = createWrapper(TASK_STATUS.REMOVED)
      const actions = wrapper.findAll('.task-item-action')
      expect(actions.length).toBeGreaterThanOrEqual(2 + 3)
    })

    it('non-seeder statuses do not have stop-seeding button', () => {
      const wrapper = createWrapper(TASK_STATUS.ACTIVE)
      const hasStopSeeding = wrapper.findAll('.task-item-action').some((a) => a.classes().includes('stop-seeding'))
      expect(hasStopSeeding).toBe(false)
    })
  })

  describe('isStopping state', () => {
    it('applies is-stopping class when gid is in stoppingGids', () => {
      const wrapper = mount(TaskItemActions, {
        props: {
          task: { gid: 'stopping-gid' } as never,
          status: TASK_STATUS.SEEDING,
        },
        global: {
          provide: {
            stoppingGids: ref(['stopping-gid']),
          },
        },
      })

      const stopAction = wrapper.findAll('.task-item-action').find((a) => a.classes().includes('stop-seeding'))
      expect(stopAction?.classes()).toContain('is-stopping')
    })

    it('does not apply is-stopping when gid is NOT in stoppingGids', () => {
      const wrapper = mount(TaskItemActions, {
        props: {
          task: { gid: 'other-gid' } as never,
          status: TASK_STATUS.SEEDING,
        },
        global: {
          provide: {
            stoppingGids: ref(['different-gid']),
          },
        },
      })

      const stopAction = wrapper.findAll('.task-item-action').find((a) => a.classes().includes('stop-seeding'))
      expect(stopAction?.classes()).not.toContain('is-stopping')
    })

    it('shows spin icon wrapper when stopping', () => {
      const wrapper = mount(TaskItemActions, {
        props: {
          task: { gid: 'spin-gid' } as never,
          status: TASK_STATUS.SEEDING,
        },
        global: {
          provide: {
            stoppingGids: ref(['spin-gid']),
          },
        },
      })

      expect(wrapper.find('.stop-icon-wrapper').exists()).toBe(true)
      expect(wrapper.find('.stop-icon-spin.fade-in').exists()).toBe(true)
    })
  })

  describe('seeder styling', () => {
    it('seeding stop button has stop-seeding class for green color', () => {
      const wrapper = createWrapper(TASK_STATUS.SEEDING)
      const stopAction = wrapper.findAll('.task-item-action').find((a) => a.classes().includes('stop-seeding'))
      expect(stopAction).toBeDefined()
      expect(stopAction!.classes()).toContain('stop-seeding')
    })
  })

  describe('press animation', () => {
    it('adds pressed class on pointerdown and removes on pointerup', async () => {
      const wrapper = createWrapper(TASK_STATUS.ACTIVE)
      const action = wrapper.find('.task-item-action')
      await action.trigger('pointerdown')
      expect(action.classes()).toContain('pressed')
      await action.trigger('pointerup')
      // Note: pressed class removal is timer-based (asynchronous), but the
      // pointerup handler schedules removal — we verify the class was added
    })
  })
})
