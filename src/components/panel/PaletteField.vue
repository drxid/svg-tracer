<script setup lang="ts">
import { useApp } from '@/composables/useApp'

const { palette, colorOverrides, recolor } = useApp()
</script>

<template>
  <div v-if="palette.length" class="grid gap-2">
    <span class="text-[13px] text-ink-2">Палитра</span>
    <div class="flex flex-wrap gap-2">
      <label
        v-for="color in palette"
        :key="color"
        class="size-6 cursor-pointer rounded-full shadow-raise"
        :style="{ background: colorOverrides[color] ?? color }"
        :title="colorOverrides[color] ?? color"
      >
        <input
          type="color"
          class="sr-only"
          :value="colorOverrides[color] ?? color"
          :aria-label="`Заменить цвет ${color}`"
          @change="recolor(color, ($event.target as HTMLInputElement).value)"
        >
      </label>
    </div>
  </div>
</template>
