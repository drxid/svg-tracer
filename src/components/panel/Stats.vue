<script setup lang="ts">
import { computed } from 'vue'
import { useApp } from '@/composables/useApp'
import { formatBytes, formatNumber } from '@/lib/stats'

const HEAVY_BYTES = 500 * 1024
const HEAVY_NODES = 20_000

const { rawStats, finalStats, optimizeOn, durationMs, optimizeWarning } = useApp()

const saved = computed(() => Math.round((1 - finalStats.value.bytes / rawStats.value.bytes) * 100))
const heavy = computed(() => finalStats.value.bytes > HEAVY_BYTES || finalStats.value.nodes > HEAVY_NODES)
</script>

<template>
  <div class="grid gap-1 font-mono text-xs tabular-nums">
    <div class="flex items-baseline justify-between">
      <span>
        <template v-if="optimizeOn">{{ formatBytes(rawStats.bytes) }} → </template>{{ formatBytes(finalStats.bytes) }}
      </span>
      <span v-if="optimizeOn && saved > 0" class="text-ok">−{{ saved }} %</span>
    </div>
    <div class="text-ink-2">
      {{ formatNumber(finalStats.nodes) }} узлов · {{ finalStats.paths }} путей · {{ finalStats.colors.length }} цв. · {{ durationMs }} мс
    </div>
    <p v-if="optimizeWarning" class="mt-1 font-sans text-warn">{{ optimizeWarning }}</p>
    <p v-if="heavy" class="mt-1 font-sans text-warn">Похоже на фото, вектор здесь тяжелее растра</p>
  </div>
</template>
