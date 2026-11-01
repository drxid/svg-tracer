<script setup lang="ts">
import type { Rect } from '@/lib/types'
import { computed, ref } from 'vue'
import { useApp } from '@/composables/useApp'

const MIN_SIDE = 4

const { image, crop, setCrop } = useApp()

const layer = ref<HTMLElement>()
const drawn = ref<Rect | null>(crop.value)

const box = computed(() => {
  const rect = drawn.value
  if (!rect)
    return null
  const { width, height } = image.value!
  return {
    left: `${rect.x / width * 100}%`,
    top: `${rect.y / height * 100}%`,
    width: `${rect.width / width * 100}%`,
    height: `${rect.height / height * 100}%`,
  }
})

function toImage(event: PointerEvent) {
  const bounds = layer.value!.getBoundingClientRect()
  const { width, height } = image.value!
  return {
    x: Math.round(Math.min(1, Math.max(0, (event.clientX - bounds.left) / bounds.width)) * width),
    y: Math.round(Math.min(1, Math.max(0, (event.clientY - bounds.top) / bounds.height)) * height),
  }
}

function start(event: PointerEvent) {
  event.stopPropagation()
  const target = event.currentTarget as HTMLElement
  const origin = toImage(event)
  target.setPointerCapture(event.pointerId)

  const onMove = (move: PointerEvent) => {
    const point = toImage(move)
    drawn.value = {
      x: Math.min(origin.x, point.x),
      y: Math.min(origin.y, point.y),
      width: Math.abs(point.x - origin.x),
      height: Math.abs(point.y - origin.y),
    }
  }
  target.addEventListener('pointermove', onMove)
  target.addEventListener('pointerup', () => {
    target.removeEventListener('pointermove', onMove)
    const rect = drawn.value
    // Щелчок без протяжки снимает кадр.
    setCrop(rect && rect.width >= MIN_SIDE && rect.height >= MIN_SIDE ? rect : null)
  }, { once: true })
}
</script>

<template>
  <div ref="layer" class="absolute inset-0 z-20 cursor-crosshair bg-black/20" @pointerdown="start">
    <div
      v-if="box"
      class="pointer-events-none absolute outline-1 outline-white shadow-[0_0_0_9999px_rgba(0,0,0,.35)]"
      :style="box"
    />
  </div>
</template>
