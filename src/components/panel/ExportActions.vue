<script setup lang="ts">
import type { ExportFormat } from '@/lib/types'
import { ChevronDown } from 'lucide-vue-next'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { useApp } from '@/composables/useApp'

const COPY_FORMATS: { value: ExportFormat, label: string }[] = [
  { value: 'svg', label: 'SVG' },
  { value: 'vue', label: 'Компонент Vue' },
  { value: 'react', label: 'Компонент React' },
  { value: 'dataUri', label: 'Data URI для CSS' },
]
const SAVE_FORMATS: { value: ExportFormat, label: string }[] = [
  { value: 'svg', label: 'Файл .svg' },
  { value: 'vue', label: 'Компонент .vue' },
  { value: 'react', label: 'Компонент .tsx' },
]

const { copy, save } = useApp()
</script>

<template>
  <div class="grid grid-cols-2 gap-2">
    <DropdownMenu>
      <DropdownMenuTrigger as-child>
        <Button variant="ghost" class="h-9 rounded-full bg-raised shadow-raise hover:bg-surface">
          Копировать
          <ChevronDown class="size-3.5 text-ink-3" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" class="w-48 rounded-xl">
        <DropdownMenuItem v-for="format in COPY_FORMATS" :key="format.value" class="rounded-lg" @select="copy(format.value)">
          {{ format.label }}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
    <DropdownMenu>
      <DropdownMenuTrigger as-child>
        <Button class="h-9 rounded-full">
          Сохранить
          <ChevronDown class="size-3.5 opacity-70" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" class="w-48 rounded-xl">
        <DropdownMenuItem v-for="format in SAVE_FORMATS" :key="format.value" class="rounded-lg" @select="save(format.value)">
          {{ format.label }}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  </div>
</template>
