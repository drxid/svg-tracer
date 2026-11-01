<script setup lang="ts">
import { Check, Download, Minus, Moon, RotateCw, Square, Sun, X } from 'lucide-vue-next'
import { computed } from 'vue'
import { Button } from '@/components/ui/button'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { useApp } from '@/composables/useApp'
import { useTheme } from '@/composables/useTheme'
import { appWindow } from '@/lib/tauri'

const { image, potrace, potraceSettled, retryModules } = useApp()
const { isDark, toggleTheme } = useTheme()

const WINDOW_BUTTONS = [
  { icon: Minus, label: 'Свернуть', action: appWindow.minimize },
  { icon: Square, label: 'Развернуть', action: appWindow.toggleMaximize },
  { icon: X, label: 'Закрыть', action: appWindow.close },
]

const pillLabel = computed(() => {
  if (potrace.value.status === 'downloading')
    return `Модули ${potrace.value.percent} %`
  return potrace.value.status === 'failed' ? 'Модуль не загружен' : 'Модули готовы'
})
</script>

<template>
  <header data-tauri-drag-region class="relative flex h-11 shrink-0 items-center border-b border-line pl-4">
    <svg viewBox="0 0 20 20" class="size-[18px]" fill="none" aria-hidden="true">
      <path d="M3.5 16.5C3.5 8 12 3.5 16.5 3.5" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" />
      <rect x="1.5" y="14.5" width="4" height="4" rx="1" fill="currentColor" />
      <rect x="14.5" y="1.5" width="4" height="4" rx="1" fill="currentColor" />
    </svg>
    <span class="ml-2.5 text-[13px] font-medium" data-tauri-drag-region>SVG Tracer</span>
    <span v-if="image" class="ml-3 truncate text-xs text-ink-2" data-tauri-drag-region>
      {{ image.name }} <span class="font-mono text-ink-3">{{ image.width }}×{{ image.height }}</span>
    </span>

    <div class="ml-auto flex items-center">
      <Popover v-if="!potraceSettled">
        <PopoverTrigger as-child>
          <button
            type="button"
            class="mr-2 flex h-7 items-center gap-1.5 rounded-full bg-surface px-3 text-xs outline-none focus-visible:ring-2 focus-visible:ring-ring"
            :class="{ 'text-danger': potrace.status === 'failed' }"
          >
            <Check v-if="potrace.status === 'ready'" class="size-3.5 text-ok" />
            <Download v-else class="size-3.5" />
            <span class="tabular-nums">{{ pillLabel }}</span>
          </button>
        </PopoverTrigger>
        <PopoverContent align="end" class="w-72 gap-0 rounded-2xl p-2">
          <div class="flex h-9 items-center justify-between rounded-lg px-2 text-[13px]">
            <span>Potrace <span class="font-mono text-xs text-ink-3">1.16</span></span>
            <span v-if="potrace.status === 'downloading'" class="font-mono text-xs tabular-nums">{{ potrace.percent }} %</span>
            <span v-else-if="potrace.status === 'ready'" class="text-xs text-ok">Готов</span>
            <button v-else-if="potrace.status === 'failed'" type="button" class="flex items-center gap-1 text-xs text-danger" @click="retryModules">
              <RotateCw class="size-3" /> Повторить
            </button>
          </div>
          <div class="flex h-9 items-center justify-between rounded-lg px-2 text-[13px]">
            <span>VTracer <span class="font-mono text-xs text-ink-3">1.0 alpha</span></span>
            <span class="text-xs text-ink-3">Встроен</span>
          </div>
          <div class="flex h-9 items-center justify-between rounded-lg px-2 text-[13px]">
            <span>SVGO <span class="font-mono text-xs text-ink-3">4.1</span></span>
            <span class="text-xs text-ink-3">Встроен</span>
          </div>
        </PopoverContent>
      </Popover>

      <Button variant="ghost" size="icon" class="rounded-full" aria-label="Сменить тему" @click="toggleTheme()">
        <Sun v-if="isDark" class="size-4" />
        <Moon v-else class="size-4" />
      </Button>
      <div class="ml-2 flex">
        <button
          v-for="control in WINDOW_BUTTONS"
          :key="control.label"
          type="button"
          :aria-label="control.label"
          class="grid h-11 w-11 place-items-center text-ink-2 transition-colors hover:bg-surface hover:text-ink"
          @click="control.action()"
        >
          <component :is="control.icon" class="size-3.5" />
        </button>
      </div>
    </div>
  </header>
</template>
