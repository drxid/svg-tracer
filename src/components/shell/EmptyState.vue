<script setup lang="ts">
import { Button } from '@/components/ui/button'
import { useApp } from '@/composables/useApp'

const { openDialog } = useApp()
</script>

<template>
  <div class="grid h-full place-content-center justify-items-center px-8 text-center">
    <h1 class="text-[40px] leading-[44px] font-light tracking-[-0.02em]">
      Перетащите картинку
    </h1>
    <p class="mt-3 text-sm text-ink-2">
      или вставьте её через <kbd class="font-mono text-xs">Ctrl+V</kbd>
    </p>
    <Button class="mt-6 h-10 rounded-full px-5 text-sm" @click="openDialog">
      Выбрать файл
    </Button>
    <p class="mt-5 text-xs text-ink-3">
      PNG, JPEG, WebP, BMP, GIF
    </p>
  </div>
</template>
