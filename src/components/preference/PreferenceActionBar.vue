<script setup lang="ts">
/** @fileoverview Shared save/discard action bar for preference pages. */
import { useI18n } from 'vue-i18n'
import { NButton, NSpace } from 'naive-ui'

defineProps<{ isDirty: boolean }>()
defineEmits<{ save: []; discard: [] }>()

const { t } = useI18n()
</script>

<template>
  <div class="form-actions">
    <NSpace>
      <NButton :class="{ 'save-btn-dirty': isDirty }" type="primary" @click="$emit('save')">
        {{ t('preferences.save') }}
      </NButton>
      <NButton :class="{ 'discard-btn-dirty': isDirty }" @click="$emit('discard')">
        {{ t('preferences.discard') }}
      </NButton>
    </NSpace>
  </div>
</template>

<style scoped>
.form-actions {
  position: sticky;
  bottom: 0;
  z-index: 10;
  display: flex;
  justify-content: flex-start;
  padding: 16px 24px 16px 40px;
}
.save-btn-dirty {
  background-color: var(--m3-success) !important;
  transition: background-color 0.35s cubic-bezier(0.2, 0, 0, 1);
}
.save-btn-dirty :deep(.n-button__border) {
  border-color: var(--m3-success) !important;
}
.save-btn-dirty :deep(.n-button__state-border) {
  border-color: var(--m3-success) !important;
}
.discard-btn-dirty {
  background-color: rgba(208, 48, 80, 0.85) !important;
  color: var(--m3-on-success) !important;
  transition:
    background-color 0.35s cubic-bezier(0.2, 0, 0, 1),
    color 0.35s cubic-bezier(0.2, 0, 0, 1);
}
.discard-btn-dirty :deep(.n-button__border) {
  border-color: rgba(208, 48, 80, 0.85) !important;
}
.discard-btn-dirty :deep(.n-button__state-border) {
  border-color: rgba(208, 48, 80, 0.85) !important;
}
</style>
