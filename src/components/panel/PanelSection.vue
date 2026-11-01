<script setup lang="ts">
import { ChevronDown } from 'lucide-vue-next'
import { CollapsibleContent, CollapsibleRoot, CollapsibleTrigger } from 'reka-ui'

defineProps<{ title: string, defaultOpen?: boolean }>()
</script>

<template>
  <CollapsibleRoot v-slot="{ open }" :default-open="defaultOpen" class="border-t border-line">
    <CollapsibleTrigger class="flex h-11 w-full items-center justify-between px-5 text-sm font-medium outline-none focus-visible:bg-surface">
      {{ title }}
      <ChevronDown class="size-4 text-ink-3 transition-transform duration-200" :class="{ 'rotate-180': open }" />
    </CollapsibleTrigger>
    <CollapsibleContent>
      <div class="grid gap-4 px-5 pt-1 pb-5">
        <slot />
      </div>
    </CollapsibleContent>
  </CollapsibleRoot>
</template>
