<script setup lang="ts">
import type { Mode } from '@/lib/types'
import { useEventListener } from '@vueuse/core'
import { onMounted, ref } from 'vue'
import { Toaster } from 'vue-sonner'
import Canvas from '@/components/canvas/Canvas.vue'
import SidePanel from '@/components/panel/SidePanel.vue'
import EmptyState from '@/components/shell/EmptyState.vue'
import TitleBar from '@/components/shell/TitleBar.vue'
import { useApp } from '@/composables/useApp'
import { useTheme } from '@/composables/useTheme'
import { MODES } from '@/lib/modes'
import { onFileDrop } from '@/lib/tauri'
import 'vue-sonner/style.css'

const VIEW_ORDER = ['split', 'result', 'source'] as const

const {
  loaded,
  running,
  view,
  cropping,
  cancelRun,
  copy,
  openBytes,
  openDialog,
  openDropped,
  redo,
  save,
  setMode,
  startup,
  undo,
} = useApp()
const { isDark } = useTheme()

const canvas = ref<InstanceType<typeof Canvas>>()
const dragging = ref(false)

onMounted(() => {
  void startup()
  void onFileDrop({
    over: () => dragging.value = true,
    leave: () => dragging.value = false,
    drop: (paths) => {
      dragging.value = false
      openDropped(paths)
    },
  })
})

useEventListener(window, 'paste', (event: ClipboardEvent) => {
  const file = event.clipboardData?.files[0]
  if (file?.type.startsWith('image/'))
    void file.arrayBuffer().then(openBytes)
})

function isTyping(target: EventTarget | null) {
  return target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement
}

const CTRL_ACTIONS: Record<string, () => void> = {
  o: () => void openDialog(),
  s: () => void save(),
  z: undo,
  0: () => canvas.value?.fit(),
  1: () => canvas.value?.zoomTo(1),
}

useEventListener(window, 'keydown', (event: KeyboardEvent) => {
  const key = event.key.toLowerCase()
  if (event.ctrlKey && event.shiftKey) {
    if (key === 'c')
      void copy('svg')
    else if (key === 'z')
      redo()
    else
      return
    event.preventDefault()
    return
  }
  if (event.ctrlKey) {
    const action = CTRL_ACTIONS[key]
    if (action) {
      event.preventDefault()
      action()
    }
    return
  }
  if (isTyping(event.target))
    return

  if (key === 'v' && loaded.value) {
    view.value = VIEW_ORDER[(VIEW_ORDER.indexOf(view.value) + 1) % VIEW_ORDER.length]!
  }
  else if (key === 'escape') {
    if (cropping.value)
      cropping.value = false
    else if (running.value)
      cancelRun()
  }
  else if (key === 'c' && loaded.value) {
    cropping.value = !cropping.value
  }
  else if (MODES[Number(key) - 1]) {
    setMode(MODES[Number(key) - 1]!.value as Mode)
  }
})
</script>

<template>
  <div class="flex h-full flex-col">
    <TitleBar />
    <div class="relative h-0.5 shrink-0 overflow-hidden">
      <span v-if="running" class="run-line" />
    </div>
    <main class="relative flex min-h-0 flex-1">
      <section class="relative min-w-0 flex-1">
        <Canvas v-if="loaded" ref="canvas" />
        <EmptyState v-else />
        <button
          v-if="running"
          type="button"
          class="absolute top-3 right-3 h-7 rounded-full bg-raised px-3 text-xs text-ink-2 shadow-raise outline-none hover:text-ink focus-visible:ring-2 focus-visible:ring-ring"
          @click="cancelRun"
        >
          Отменить
        </button>
        <div v-if="dragging" class="pointer-events-none absolute inset-3 grid place-items-center rounded-2xl bg-surface outline-1 outline-line">
          <p class="text-[40px] leading-[44px] font-light tracking-[-0.02em]">
            Отпустите файл
          </p>
        </div>
      </section>
      <SidePanel />
    </main>
    <Toaster :theme="isDark ? 'dark' : 'light'" position="bottom-left" />
  </div>
</template>
