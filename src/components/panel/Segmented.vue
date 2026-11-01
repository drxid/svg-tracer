<script setup lang="ts" generic="T extends string">
import { ToggleGroupItem, ToggleGroupRoot } from 'reka-ui'
import { computed } from 'vue'

const props = defineProps<{
  modelValue: T
  options: { value: T, label: string }[]
  size?: 'md' | 'sm'
}>()
const emit = defineEmits<{ 'update:modelValue': [value: T] }>()

const index = computed(() => props.options.findIndex(option => option.value === props.modelValue))

function select(value: unknown) {
  if (value)
    emit('update:modelValue', value as T)
}
</script>

<template>
  <ToggleGroupRoot
    type="single"
    :model-value="modelValue"
    class="relative grid rounded-xl bg-surface p-1"
    :style="{ gridTemplateColumns: `repeat(${options.length}, minmax(0, 1fr))` }"
    @update:model-value="select"
  >
    <span
      class="absolute inset-y-1 left-1 rounded-[9px] bg-raised shadow-raise transition-transform duration-200 ease-ui"
      :style="{ width: `calc((100% - 8px) / ${options.length})`, transform: `translateX(${index * 100}%)` }"
    />
    <ToggleGroupItem
      v-for="option in options"
      :key="option.value"
      :value="option.value"
      class="relative z-10 truncate rounded-[9px] px-1.5 text-ink-2 outline-none transition-colors focus-visible:ring-2 focus-visible:ring-ring data-[state=on]:text-ink"
      :class="size === 'sm' ? 'h-7 text-xs' : 'h-9 text-[13px]'"
    >
      {{ option.label }}
    </ToggleGroupItem>
  </ToggleGroupRoot>
</template>
