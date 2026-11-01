<script setup lang="ts">
import type { CompareView } from '@/lib/types'
import { Crop, Grid2x2, Image, Maximize, Minus, Plus, ScanLine, SplitSquareHorizontal, Spline } from 'lucide-vue-next'
import { ToggleGroupItem, ToggleGroupRoot } from 'reka-ui'
import { useApp } from '@/composables/useApp'

defineProps<{ percent: number }>()
const emit = defineEmits<{ zoomIn: [], zoomOut: [], fit: [] }>()

const VIEWS = [
  { value: 'split', icon: SplitSquareHorizontal, label: 'Шторка' },
  { value: 'result', icon: Spline, label: 'Только SVG' },
  { value: 'source', icon: Image, label: 'Только оригинал' },
] as const

const BACKGROUNDS = ['checker', 'white', 'black'] as const

const { view, canvasBackground, cropping, crop, showOutlines } = useApp()

function selectView(value: unknown) {
  if (value)
    view.value = value as CompareView
}

function nextBackground() {
  const index = BACKGROUNDS.indexOf(canvasBackground.value)
  canvasBackground.value = BACKGROUNDS[(index + 1) % BACKGROUNDS.length]!
}

const iconButton = 'grid size-8 place-items-center rounded-full text-ink-2 outline-none transition-colors hover:bg-surface hover:text-ink focus-visible:ring-2 focus-visible:ring-ring'
const activeButton = 'bg-surface-2 text-ink'
</script>

<template>
  <div class="flex h-11 items-center gap-1 rounded-full bg-raised px-1.5 shadow-raise">
    <ToggleGroupRoot type="single" :model-value="view" class="flex gap-0.5" @update:model-value="selectView">
      <ToggleGroupItem
        v-for="item in VIEWS"
        :key="item.value"
        :value="item.value"
        :title="item.label"
        :aria-label="item.label"
        :class="[iconButton, 'data-[state=on]:bg-surface-2 data-[state=on]:text-ink']"
      >
        <component :is="item.icon" class="size-4" />
      </ToggleGroupItem>
    </ToggleGroupRoot>
    <span class="mx-1 h-5 w-px bg-line" />
    <button type="button" :class="iconButton" title="Уменьшить" aria-label="Уменьшить" @click="emit('zoomOut')">
      <Minus class="size-4" />
    </button>
    <span class="w-12 text-center font-mono text-xs tabular-nums">{{ percent }} %</span>
    <button type="button" :class="iconButton" title="Увеличить" aria-label="Увеличить" @click="emit('zoomIn')">
      <Plus class="size-4" />
    </button>
    <button type="button" :class="iconButton" title="По размеру" aria-label="По размеру" @click="emit('fit')">
      <Maximize class="size-4" />
    </button>
    <span class="mx-1 h-5 w-px bg-line" />
    <button type="button" :class="iconButton" title="Фон холста" aria-label="Фон холста" @click="nextBackground">
      <Grid2x2 class="size-4" />
    </button>
    <button
      type="button"
      :class="[iconButton, showOutlines && activeButton]"
      title="Показать контуры"
      aria-label="Показать контуры"
      :aria-pressed="showOutlines"
      @click="showOutlines = !showOutlines"
    >
      <ScanLine class="size-4" />
    </button>
    <button
      type="button"
      :class="[iconButton, (cropping || crop) && activeButton]"
      title="Кадр (C)"
      aria-label="Кадр"
      :aria-pressed="cropping"
      @click="cropping = !cropping"
    >
      <Crop class="size-4" />
    </button>
  </div>
</template>
