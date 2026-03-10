<script setup lang="ts">
/** @fileoverview Batch task action buttons: resume all, pause all, delete all, purge. */
import { ref, computed, h } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAppStore } from '@/stores/app'
import { useTaskStore } from '@/stores/task'

import { isEngineReady } from '@/api/aria2'
import { deleteTaskFiles } from '@/composables/useFileDelete'
import { logger } from '@shared/logger'
import { NButton, NIcon, NTooltip, NCheckbox, useDialog } from 'naive-ui'
import { useAppMessage } from '@/composables/useAppMessage'
import { AddOutline, PlayOutline, PauseOutline, TrashOutline, RefreshOutline, CloseOutline } from '@vicons/ionicons5'

const { t } = useI18n()
const appStore = useAppStore()
const taskStore = useTaskStore()
const message = useAppMessage()
const dialog = useDialog()

const refreshing = ref(false)
let refreshTimer: ReturnType<typeof setTimeout> | null = null

const currentList = computed(() => taskStore.currentList)
const allGids = computed(() => taskStore.taskList.map((t: { gid: string }) => t.gid))

function showAddTask() {
  appStore.showAddTaskDialog()
}

function onRefresh() {
  if (refreshTimer) clearTimeout(refreshTimer)
  refreshing.value = true
  refreshTimer = setTimeout(() => {
    refreshing.value = false
  }, 500)
  taskStore
    .fetchList()
    .then(() => message.success(t('task.refresh-list-success') || 'List refreshed'))
    .catch((e: unknown) => logger.warn('TaskActions.onRefresh', (e as Error).message))
}

function onDeleteAll() {
  if (allGids.value.length === 0) return
  const gids = [...allGids.value]
  const deleteFiles = ref(false)
  const d = dialog.warning({
    title: t('task.delete-task'),
    content: () =>
      h('div', {}, [
        h('p', { style: 'margin: 0 0 12px;' }, t('task.batch-delete-task-confirm', { count: gids.length })),
        h(
          NCheckbox,
          {
            checked: deleteFiles.value,
            'onUpdate:checked': (v: boolean) => {
              deleteFiles.value = v
            },
          },
          { default: () => t('task.delete-task-label') },
        ),
      ]),
    positiveText: t('app.yes'),
    negativeText: t('app.no'),
    onPositiveClick: async () => {
      d.loading = true
      d.negativeButtonProps = { disabled: true }
      d.closable = false
      d.maskClosable = false
      // Yield to browser so the loading spinner renders before heavy IPC work
      await new Promise((r) => setTimeout(r, 50))
      // Capture task references BEFORE removal — the store list mutates after
      // batchRemoveTask, so we'd lose the dir/path info needed for file deletion.
      const tasksToDelete = deleteFiles.value ? taskStore.taskList.filter((t) => gids.includes(t.gid)) : []
      // Remove task records FIRST, then delete files.
      // This matches the safer order used in single-task delete (TaskView.vue).
      // If file deletion fails, tasks are already cleaned up from aria2;
      // the reverse order would leave orphaned tasks with missing files.
      await taskStore.batchRemoveTask(gids)
      for (const task of tasksToDelete) {
        await deleteTaskFiles(task)
      }
      message.success(t('task.batch-delete-task-success'))
    },
  })
}

function resumeAll() {
  if (!isEngineReady()) {
    message.warning(t('app.engine-not-ready'))
    return
  }
  dialog.warning({
    title: t('task.resume-all-task'),
    content: t('task.resume-all-task-confirm') || 'Resume all tasks?',
    positiveText: t('app.yes'),
    negativeText: t('app.no'),
    onPositiveClick: async () => {
      await taskStore
        .resumeAllTask()
        .then(() => message.success(t('task.resume-all-task-success')))
        .catch(() => message.error(t('task.resume-all-task-fail')))
    },
  })
}

function pauseAll() {
  if (!isEngineReady()) {
    message.warning(t('app.engine-not-ready'))
    return
  }
  dialog.warning({
    title: t('task.pause-all-task'),
    content: t('task.pause-all-task-confirm') || 'Pause all tasks?',
    positiveText: t('app.yes'),
    negativeText: t('app.no'),
    onPositiveClick: async () => {
      await taskStore
        .pauseAllTask()
        .then(() => message.success(t('task.pause-all-task-success')))
        .catch(() => message.error(t('task.pause-all-task-fail')))
    },
  })
}

function purgeRecord() {
  if (!isEngineReady()) {
    message.warning(t('app.engine-not-ready'))
    return
  }
  dialog.warning({
    title: t('task.purge-record'),
    content: t('task.purge-record-confirm') || 'Clear all finished records?',
    positiveText: t('app.yes'),
    negativeText: t('app.no'),
    onPositiveClick: async () => {
      await taskStore
        .purgeTaskRecord()
        .then(() => message.success(t('task.purge-record-success')))
        .catch(() => message.error(t('task.purge-record-fail')))
    },
  })
}

/** M3 press/release animation for toolbar buttons */
const MIN_PRESS_MS = 200
const pressTimers = new WeakMap<HTMLElement, { start: number; timer: ReturnType<typeof setTimeout> | null }>()

function onBtnPress(ev: PointerEvent) {
  const el = ev.currentTarget as HTMLElement
  const prev = pressTimers.get(el)
  if (prev?.timer) clearTimeout(prev.timer)
  el.classList.add('pressed')
  pressTimers.set(el, { start: Date.now(), timer: null })
}

function onBtnRelease(ev: PointerEvent) {
  const el = ev.currentTarget as HTMLElement
  const state = pressTimers.get(el)
  if (!state) {
    el.classList.remove('pressed')
    return
  }
  const elapsed = Date.now() - state.start
  const remaining = Math.max(0, MIN_PRESS_MS - elapsed)
  state.timer = setTimeout(() => {
    el.classList.remove('pressed')
    pressTimers.delete(el)
  }, remaining)
}
</script>

<template>
  <div class="task-actions">
    <NTooltip>
      <template #trigger>
        <NButton
          type="primary"
          circle
          size="small"
          @pointerdown="onBtnPress"
          @pointerup="onBtnRelease"
          @pointerleave="onBtnRelease"
          @click="showAddTask"
        >
          <template #icon>
            <NIcon><AddOutline /></NIcon>
          </template>
        </NButton>
      </template>
      {{ t('task.new-task') || 'New Task' }}
    </NTooltip>
    <NTooltip>
      <template #trigger>
        <NButton
          quaternary
          circle
          size="small"
          @pointerdown="onBtnPress"
          @pointerup="onBtnRelease"
          @pointerleave="onBtnRelease"
          @click="onRefresh"
        >
          <template #icon>
            <NIcon :class="{ spinning: refreshing }"><RefreshOutline /></NIcon>
          </template>
        </NButton>
      </template>
      {{ t('task.refresh-list') || 'Refresh' }}
    </NTooltip>
    <NTooltip v-if="currentList !== 'stopped'">
      <template #trigger>
        <NButton
          quaternary
          circle
          size="small"
          @pointerdown="onBtnPress"
          @pointerup="onBtnRelease"
          @pointerleave="onBtnRelease"
          @click="resumeAll"
        >
          <template #icon>
            <NIcon><PlayOutline /></NIcon>
          </template>
        </NButton>
      </template>
      {{ t('task.resume-all-task') || 'Resume All' }}
    </NTooltip>
    <NTooltip v-if="currentList !== 'stopped'">
      <template #trigger>
        <NButton
          quaternary
          circle
          size="small"
          @pointerdown="onBtnPress"
          @pointerup="onBtnRelease"
          @pointerleave="onBtnRelease"
          @click="pauseAll"
        >
          <template #icon>
            <NIcon><PauseOutline /></NIcon>
          </template>
        </NButton>
      </template>
      {{ t('task.pause-all-task') || 'Pause All' }}
    </NTooltip>
    <NTooltip v-if="currentList !== 'stopped'">
      <template #trigger>
        <NButton
          quaternary
          circle
          size="small"
          :disabled="allGids.length === 0"
          @pointerdown="onBtnPress"
          @pointerup="onBtnRelease"
          @pointerleave="onBtnRelease"
          @click="onDeleteAll"
        >
          <template #icon>
            <NIcon><CloseOutline /></NIcon>
          </template>
        </NButton>
      </template>
      {{ t('task.delete-all-task') }}
    </NTooltip>
    <NTooltip v-if="currentList === 'stopped'">
      <template #trigger>
        <NButton
          quaternary
          circle
          size="small"
          @pointerdown="onBtnPress"
          @pointerup="onBtnRelease"
          @pointerleave="onBtnRelease"
          @click="purgeRecord"
        >
          <template #icon>
            <NIcon><TrashOutline /></NIcon>
          </template>
        </NButton>
      </template>
      {{ t('task.purge-record') || 'Purge Records' }}
    </NTooltip>
  </div>
</template>

<style scoped>
.task-actions {
  display: flex;
  gap: 4px;
  align-items: center;
}
.task-actions :deep(.n-button) {
  transition: transform 0.25s cubic-bezier(0.05, 0.7, 0.1, 1);
  transform-origin: center;
}
.task-actions :deep(.n-button.pressed) {
  transform: scale(0.85);
  transition: transform 0.2s cubic-bezier(0.2, 0, 0, 1);
}
@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}
.spinning {
  animation: spin 0.6s cubic-bezier(0.2, 0, 0, 1);
  display: inline-block;
  transform-origin: center;
}
</style>
