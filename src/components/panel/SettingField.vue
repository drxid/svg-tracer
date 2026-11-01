<script setup lang="ts">
import type { SettingDef } from '@/lib/types'
import { computed } from 'vue'
import { Slider } from '@/components/ui/slider'
import { formatNumber } from '@/lib/stats'
import Segmented from './Segmented.vue'

const props = defineProps<{ def: SettingDef, modelValue: number | string }>()
const emit = defineEmits<{ 'update:modelValue': [value: number | string] }>()

const sliderValue = computed(() => [Number(props.modelValue)])

const display = computed(() => {
  const { def } = props
  if (def.kind !== 'slider')
    return ''
  const value = Number(props.modelValue)
  if (def.zeroOff && value === 0)
    return 'выкл.'
  return `${formatNumber(value)}${def.unit ? ` ${def.unit}` : ''}`
})

function resetToDefault() {
  emit('update:modelValue', props.def.value)
}
</script>

<template>
  <div v-if="def.kind === 'slider'" class="grid gap-2.5">
    <div class="flex items-baseline justify-between">
      <span class="text-[13px] text-ink-2" title="Двойной клик вернёт значение по умолчанию" @dblclick="resetToDefault">{{ def.label }}</span>
      <span class="font-mono text-xs tabular-nums text-ink">{{ display }}</span>
    </div>
    <Slider
      :model-value="sliderValue"
      :min="def.min"
      :max="def.max"
      :step="def.step"
      :aria-label="def.label"
      @update:model-value="emit('update:modelValue', $event?.[0] ?? def.value)"
    />
  </div>
  <label v-else-if="def.kind === 'color'" class="flex items-center justify-between">
    <span class="text-[13px] text-ink-2">{{ def.label }}</span>
    <input
      type="color"
      class="size-6 cursor-pointer rounded-full"
      :aria-label="def.label"
      :value="modelValue"
      @change="emit('update:modelValue', ($event.target as HTMLInputElement).value)"
    >
  </label>
  <div v-else class="grid gap-2">
    <span class="text-[13px] text-ink-2">{{ def.label }}</span>
    <Segmented
      size="sm"
      :model-value="String(modelValue)"
      :options="def.options"
      @update:model-value="emit('update:modelValue', $event)"
    />
  </div>
</template>
