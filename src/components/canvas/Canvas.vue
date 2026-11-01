<script setup lang="ts">
import { useElementSize } from '@vueuse/core'
import { GripVertical } from 'lucide-vue-next'
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { useApp } from '@/composables/useApp'
import { asOutlines } from '@/lib/svg'
import CropFrame from './CropFrame.vue'
import ViewToolbar from './ViewToolbar.vue'

const MIN_ZOOM = 0.1
const MAX_ZOOM = 16
const ZOOM_STEP = 1.25
const FIT_GAP_X = 96
const FIT_GAP_Y = 136
/** Холст центрирован с отступом снизу под панель инструментов. */
const CENTER_SHIFT_Y = 20
const KEY_STEP = 1

const { image, sourceUrl, resultUrl, bounds, final, view, canvasBackground, showOutlines, cropping } = useApp()

const viewport = ref<HTMLElement>()
const artwork = ref<HTMLElement>()
const { width, height } = useElementSize(viewport)

const zoom = ref<number | null>(null)
const offset = ref({ x: 0, y: 0 })
const split = ref(50)

const imageWidth = computed(() => image.value?.width ?? 1)
const imageHeight = computed(() => image.value?.height ?? 1)
const fitZoom = computed(() => Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, Math.min(
  (width.value - FIT_GAP_X) / imageWidth.value,
  (height.value - FIT_GAP_Y) / imageHeight.value,
))))
const scale = computed(() => zoom.value ?? fitZoom.value)
const displayWidth = computed(() => Math.round(imageWidth.value * scale.value))
const displayHeight = computed(() => Math.round(imageHeight.value * scale.value))
const backgroundClass = computed(() => ({
  checker: 'bg-checker',
  white: 'bg-white',
  black: 'bg-black',
})[canvasBackground.value])

/** Результат может быть обрезан по рисунку: ставим его туда, где он был в исходнике. */
const resultBox = computed(() => {
  const box = bounds.value
  if (!box)
    return { inset: '0' }
  return {
    left: `${box.x / imageWidth.value * 100}%`,
    top: `${box.y / imageHeight.value * 100}%`,
    width: `${box.width / imageWidth.value * 100}%`,
    height: `${box.height / imageHeight.value * 100}%`,
  }
})

const outlinesUrl = ref('')
watch([showOutlines, final], ([show, svg]) => {
  if (outlinesUrl.value)
    URL.revokeObjectURL(outlinesUrl.value)
  outlinesUrl.value = show && svg ? URL.createObjectURL(new Blob([asOutlines(svg)], { type: 'image/svg+xml' })) : ''
})
onBeforeUnmount(() => outlinesUrl.value && URL.revokeObjectURL(outlinesUrl.value))

function fit() {
  zoom.value = null
  offset.value = { x: 0, y: 0 }
}

function zoomTo(next: number, anchor?: { x: number, y: number }) {
  const clamped = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, next))
  if (anchor) {
    // Точка под курсором остаётся на месте.
    const ratio = clamped / scale.value
    offset.value = {
      x: anchor.x - (anchor.x - offset.value.x) * ratio,
      y: anchor.y - (anchor.y - offset.value.y) * ratio,
    }
  }
  zoom.value = clamped
}

function onWheel(event: WheelEvent) {
  event.preventDefault()
  if (!event.ctrlKey) {
    offset.value = { x: offset.value.x - event.deltaX, y: offset.value.y - event.deltaY }
    return
  }
  const box = viewport.value!.getBoundingClientRect()
  const anchor = {
    x: event.clientX - box.left - box.width / 2,
    y: event.clientY - box.top - box.height / 2 + CENTER_SHIFT_Y,
  }
  zoomTo(scale.value * (event.deltaY < 0 ? ZOOM_STEP : 1 / ZOOM_STEP), anchor)
}

function trackPointer(event: PointerEvent, onMove: (move: PointerEvent) => void) {
  const target = event.currentTarget as HTMLElement
  target.setPointerCapture(event.pointerId)
  target.addEventListener('pointermove', onMove)
  target.addEventListener('pointerup', () => target.removeEventListener('pointermove', onMove), { once: true })
}

function startPan(event: PointerEvent) {
  const start = { x: event.clientX - offset.value.x, y: event.clientY - offset.value.y }
  trackPointer(event, (move) => {
    offset.value = { x: move.clientX - start.x, y: move.clientY - start.y }
  })
}

function startSplit(event: PointerEvent) {
  event.stopPropagation()
  trackPointer(event, (move) => {
    const box = artwork.value!.getBoundingClientRect()
    split.value = Math.min(100, Math.max(0, (move.clientX - box.left) / box.width * 100))
  })
}

function moveSplit(step: number) {
  split.value = Math.min(100, Math.max(0, split.value + step))
}

defineExpose({ fit, zoomTo })
</script>

<template>
  <div
    ref="viewport"
    class="relative h-full cursor-grab touch-none overflow-hidden active:cursor-grabbing"
    @wheel="onWheel"
    @pointerdown="startPan"
  >
    <div class="absolute inset-0 grid place-items-center pb-10">
      <div
        ref="artwork"
        class="relative overflow-hidden rounded-[4px] shadow-raise"
        :class="backgroundClass"
        :style="{ width: `${displayWidth}px`, height: `${displayHeight}px`, transform: `translate(${offset.x}px, ${offset.y}px)` }"
      >
        <img
          v-if="view !== 'result'"
          :src="sourceUrl"
          alt="Исходник"
          draggable="false"
          class="absolute inset-0 size-full"
        >
        <!-- Отдельный слой на всю площадь: иначе слева от шторки и вне рисунка виден разный фон. -->
        <div
          v-if="view !== 'source'"
          class="absolute inset-0"
          :class="backgroundClass"
          :style="view === 'split' ? { clipPath: `inset(0 0 0 ${split}%)` } : undefined"
        >
          <img
            v-if="resultUrl"
            :src="resultUrl"
            alt="Результат"
            draggable="false"
            class="absolute"
            :style="resultBox"
          >
          <img
            v-if="outlinesUrl"
            :src="outlinesUrl"
            alt=""
            draggable="false"
            class="absolute"
            :style="resultBox"
          >
        </div>
        <button
          v-if="view === 'split' && resultUrl"
          type="button"
          aria-label="Шторка сравнения"
          class="absolute inset-y-0 z-10 w-6 -translate-x-1/2 cursor-ew-resize outline-none"
          :style="{ left: `${split}%` }"
          @pointerdown="startSplit"
          @keydown.left.prevent="moveSplit(-KEY_STEP)"
          @keydown.right.prevent="moveSplit(KEY_STEP)"
        >
          <span class="absolute inset-y-0 left-1/2 w-px -translate-x-1/2 bg-black/60" />
          <span class="absolute top-1/2 left-1/2 grid h-8 w-5 -translate-1/2 place-items-center rounded-full bg-raised shadow-raise">
            <GripVertical class="size-3.5 text-ink-2" />
          </span>
        </button>
        <CropFrame v-if="cropping" />
      </div>
    </div>

    <div class="absolute inset-x-0 bottom-5 flex justify-center" @pointerdown.stop>
      <ViewToolbar
        :percent="Math.round(scale * 100)"
        @zoom-in="zoomTo(scale * ZOOM_STEP)"
        @zoom-out="zoomTo(scale / ZOOM_STEP)"
        @fit="fit"
      />
    </div>
  </div>
</template>
