<script setup lang="ts">
/**
 * @fileoverview Platform-aware window control buttons.
 *
 * On macOS, native traffic lights are provided by the OS via
 * `titleBarStyle: "Overlay"` in tauri.macos.conf.json — this component
 * renders nothing.
 *
 * On Windows/Linux, renders circular icon-only buttons that share the
 * same visual language as the sidebar navigation icons (32 px circles,
 * transparent background, M3 state-layer hover).  Close uses the M3
 * `error` role on hover for semantic consistency with destructive
 * actions throughout the app.
 *
 * Icons are inline SVG (10 × 10 viewport, 1.2 px stroke, round caps)
 * instead of Ionicons — thinner, crisper, and zero external dependency.
 */
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { usePreferenceStore } from '@/stores/preference'

const props = defineProps<{
  isMaximized: boolean
  /** Current OS platform identifier (e.g. 'macos', 'windows', 'linux'). */
  platform: string
}>()

const emit = defineEmits<{
  close: []
  'maximize-toggled': []
}>()

const appWindow = getCurrentWindow()
const preferenceStore = usePreferenceStore()

/** macOS uses native traffic lights — hide custom controls entirely. */
const isMac = computed(() => props.platform === 'macos')

// ── Window focus state ──────────────────────────────────────────────
const isFocused = ref(true)
let unlistenFocus: (() => void) | null = null

onMounted(async () => {
  if (!isMac.value) {
    unlistenFocus = await appWindow.onFocusChanged(({ payload }) => {
      isFocused.value = payload
    })
  }
})

onUnmounted(() => {
  unlistenFocus?.()
})

// ── Window actions ──────────────────────────────────────────────────

function minimize() {
  appWindow.minimize()
}

function toggleMaximize() {
  appWindow.toggleMaximize()
  emit('maximize-toggled')
}

async function close() {
  if (preferenceStore.config.minimizeToTrayOnClose) {
    const { invoke } = await import('@tauri-apps/api/core')
    await invoke('set_dock_visible', { visible: false })
    appWindow.hide()
  } else {
    emit('close')
  }
}
</script>

<template>
  <!-- macOS: native traffic lights provided by OS, render nothing -->
  <div v-if="!isMac" class="caption-bar">
    <button class="caption-btn" :class="{ unfocused: !isFocused }" title="Minimize" @click="minimize">
      <!-- Minimize: horizontal stroke -->
      <svg width="10" height="10" viewBox="0 0 10 10" fill="none" aria-hidden="true">
        <path d="M2 5h6" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" />
      </svg>
    </button>
    <button
      class="caption-btn"
      :class="{ unfocused: !isFocused }"
      :title="isMaximized ? 'Restore' : 'Maximize'"
      @click="toggleMaximize"
    >
      <!-- Maximize ↔ Restore: rounded rect vs stacked rects -->
      <Transition name="icon-swap" mode="out-in">
        <svg v-if="isMaximized" key="restore" width="10" height="10" viewBox="0 0 10 10" fill="none" aria-hidden="true">
          <rect x="2.6" y="0.6" width="6.8" height="6.8" rx="1" stroke="currentColor" stroke-width="1.2" />
          <path
            d="M0.6 3.2v4.7a1.5 1.5 0 001.5 1.5H6.8"
            stroke="currentColor"
            stroke-width="1.2"
            stroke-linecap="round"
          />
        </svg>
        <svg v-else key="maximize" width="10" height="10" viewBox="0 0 10 10" fill="none" aria-hidden="true">
          <rect x="0.6" y="0.6" width="8.8" height="8.8" rx="1.5" stroke="currentColor" stroke-width="1.2" />
        </svg>
      </Transition>
    </button>
    <button class="caption-btn caption-close" :class="{ unfocused: !isFocused }" title="Close" @click="close">
      <!-- Close: diagonal cross -->
      <svg width="10" height="10" viewBox="0 0 10 10" fill="none" aria-hidden="true">
        <path d="M2 2l6 6M8 2L2 8" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" />
      </svg>
    </button>
  </div>
</template>

<style scoped>
/* ─────────────────────────────────────────────────────────────────────
 * Window caption buttons — same design family as AsideBar .menu-button:
 *   32 px circle · transparent bg · M3 on-surface-variant icon color
 *   hover: M3 state-layer 8% · close: M3 error role
 * ────────────────────────────────────────────────────────────────── */
.caption-bar {
  position: fixed;
  top: 8px;
  right: 10px;
  display: flex;
  align-items: center;
  gap: 4px;
  z-index: 9999;
}

.caption-btn {
  width: 32px;
  height: 32px;
  border: none;
  border-radius: 16px;
  background: transparent;
  color: var(--m3-on-surface-variant);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition:
    background-color 0.2s cubic-bezier(0.2, 0, 0, 1),
    color 0.2s cubic-bezier(0.2, 0, 0, 1);
  outline: none;
  padding: 0;
}

.caption-btn:hover {
  background: color-mix(in srgb, var(--m3-on-surface) 8%, transparent);
  color: var(--m3-on-surface);
}

/* Close button: M3 error role for semantic destructive action */
.caption-btn.caption-close:hover {
  background: var(--m3-error);
  color: var(--m3-on-error);
}

/* ── Unfocused state — dim icons to match OS convention ─────────── */
.caption-btn.unfocused {
  color: var(--m3-outline);
}

.caption-btn.unfocused:hover {
  color: var(--m3-on-surface);
}

.caption-btn.caption-close.unfocused:hover {
  color: var(--m3-on-error);
}

/* ── Maximize ↔ Restore icon cross-fade ─────────────────────────── */
.icon-swap-enter-active,
.icon-swap-leave-active {
  transition:
    opacity 150ms ease,
    transform 150ms ease;
}

.icon-swap-enter-from {
  opacity: 0;
  transform: scale(0.75);
}

.icon-swap-leave-to {
  opacity: 0;
  transform: scale(0.75);
}
</style>
