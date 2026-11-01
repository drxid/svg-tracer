<script setup lang="ts">
import type { Component } from 'vue'
import type { Preset } from '@/lib/types'
import { Aperture, Bookmark, Box, ChevronDown, Grid3x3, Hexagon, Image, Palette, Plus, ScanLine, Shapes, X } from 'lucide-vue-next'
import { computed, nextTick, ref } from 'vue'
import { useApp } from '@/composables/useApp'

const VISIBLE = 4
const ICONS: Record<string, Component> = {
  logo: Hexagon,
  icon: Shapes,
  illustration: Palette,
  scan: ScanLine,
  geometry: Box,
  poster: Aperture,
  pixel: Grid3x3,
  photo: Image,
}

const { presets, activePreset, applyPreset, addPreset, removePreset } = useApp()
const expanded = ref(false)
const naming = ref(false)
const draft = ref('')
const input = ref<HTMLInputElement>()

const shown = computed<Preset[]>(() => {
  if (expanded.value)
    return presets.value
  const head = presets.value.slice(0, VISIBLE)
  const active = presets.value.find(preset => preset.id === activePreset.value)
  return active && !head.includes(active) ? [...head.slice(0, -1), active] : head
})

async function startNaming() {
  naming.value = true
  await nextTick()
  input.value?.focus()
}

function finishNaming() {
  const name = draft.value.trim()
  if (name)
    addPreset(name)
  draft.value = ''
  naming.value = false
}
</script>

<template>
  <div class="grid gap-0.5 px-3 pb-3">
    <div v-for="preset in shown" :key="preset.id" class="group relative">
      <button
        type="button"
        class="flex h-10 w-full items-center gap-2.5 rounded-xl px-2 text-left outline-none transition-colors hover:bg-surface focus-visible:ring-2 focus-visible:ring-ring"
        :class="{ 'bg-surface-2 hover:bg-surface-2': preset.id === activePreset }"
        @click="applyPreset(preset)"
      >
        <span class="grid size-6 place-items-center rounded-md" :class="preset.id === activePreset ? 'bg-ink text-bg' : 'text-ink-2'">
          <component :is="ICONS[preset.id] ?? Bookmark" class="size-4" />
        </span>
        <span class="truncate text-[13px]">{{ preset.name }}</span>
        <span class="ml-auto shrink-0 text-xs text-ink-3" :class="{ 'group-hover:opacity-0': preset.custom }">{{ preset.hint }}</span>
      </button>
      <button
        v-if="preset.custom"
        type="button"
        class="absolute top-2 right-2 grid size-6 place-items-center rounded-full text-ink-3 opacity-0 outline-none hover:text-danger focus-visible:opacity-100 group-hover:opacity-100"
        :aria-label="`Удалить пресет ${preset.name}`"
        @click="removePreset(preset.id)"
      >
        <X class="size-3.5" />
      </button>
    </div>
    <div class="flex h-8 items-center justify-between">
      <button
        type="button"
        class="flex h-8 items-center gap-1 rounded-lg px-2 text-xs text-ink-2 outline-none hover:text-ink focus-visible:ring-2 focus-visible:ring-ring"
        @click="expanded = !expanded"
      >
        {{ expanded ? 'Свернуть' : 'Ещё' }}
        <ChevronDown class="size-3.5 transition-transform" :class="{ 'rotate-180': expanded }" />
      </button>
      <input
        v-if="naming"
        ref="input"
        v-model="draft"
        aria-label="Название пресета"
        placeholder="Название"
        class="h-7 w-36 rounded-lg bg-surface px-2 text-xs outline-none focus-visible:ring-2 focus-visible:ring-ring"
        @keydown.enter="finishNaming"
        @keydown.esc="naming = false"
        @blur="finishNaming"
      >
      <button
        v-else
        type="button"
        class="flex h-8 items-center gap-1 rounded-lg px-2 text-xs text-ink-2 outline-none hover:text-ink focus-visible:ring-2 focus-visible:ring-ring"
        @click="startNaming"
      >
        <Plus class="size-3.5" />
        Сохранить как пресет
      </button>
    </div>
  </div>
</template>
