<script setup lang="ts">
import { Button } from '@/components/ui/button'
import { useApp } from '@/composables/useApp'

const { potrace, retryModules } = useApp()
</script>

<template>
  <div class="mx-3 mb-3 grid gap-3 rounded-2xl bg-surface p-4">
    <template v-if="potrace.status === 'downloading'">
      <p class="text-[13px]">Загружается Potrace</p>
      <p class="text-xs leading-relaxed text-ink-2">Этот режим включится сам, как только модуль будет готов. «Цвет» уже работает.</p>
      <div class="h-1 overflow-hidden rounded-full bg-surface-2">
        <div class="h-full rounded-full bg-ink transition-[width] duration-150" :style="{ width: `${potrace.percent}%` }" />
      </div>
    </template>
    <template v-else-if="potrace.status === 'failed'">
      <p class="text-[13px]">Potrace не загрузился</p>
      <p class="text-xs leading-relaxed text-ink-2">{{ potrace.message }}. «Цвет» работает и без него.</p>
      <Button size="lg" class="h-9 justify-self-start rounded-full px-4" @click="retryModules">
        Повторить
      </Button>
    </template>
    <p v-else class="text-[13px] text-ink-2">Проверяю Potrace…</p>
  </div>
</template>
