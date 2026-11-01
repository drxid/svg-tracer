<script setup lang="ts">
import type { Mode } from '@/lib/types'
import { computed } from 'vue'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Switch } from '@/components/ui/switch'
import { useApp } from '@/composables/useApp'
import { isVisible, MODE_SETTINGS, MODES, PREPROCESS_SETTINGS } from '@/lib/modes'
import ExportActions from './ExportActions.vue'
import ModeLocked from './ModeLocked.vue'
import PaletteField from './PaletteField.vue'
import PanelSection from './PanelSection.vue'
import Presets from './Presets.vue'
import Segmented from './Segmented.vue'
import SettingField from './SettingField.vue'
import Stats from './Stats.vue'

const {
  loaded,
  mode,
  modeLocked,
  values,
  preprocess,
  optimizeOn,
  precision,
  multipass,
  removeDimensions,
  final,
  runError,
  dirty,
  run,
  setMode,
  setValue,
} = useApp()

const settings = computed(() => MODE_SETTINGS[mode.value].filter(def => isVisible(def, values[mode.value], preprocess[mode.value])))
const preprocessSettings = computed(() => PREPROCESS_SETTINGS.filter(def => isVisible(def, values[mode.value], preprocess[mode.value])))
</script>

<template>
  <aside class="flex w-[340px] shrink-0 flex-col border-l border-line max-[1100px]:w-[300px]">
    <div class="min-h-0 flex-1" :class="{ 'pointer-events-none opacity-40': !loaded }">
      <ScrollArea class="h-full">
        <div class="p-3">
          <Segmented :model-value="mode" :options="MODES" @update:model-value="setMode($event as Mode)" />
        </div>
        <Presets />
        <ModeLocked v-if="modeLocked" />
        <template v-else>
          <PanelSection title="Настройки" default-open>
            <SettingField
              v-for="def in settings"
              :key="`${mode}-${def.key}`"
              :def="def"
              :model-value="values[mode][def.key]!"
              @update:model-value="setValue(values[mode], def.key, $event)"
            />
            <PaletteField />
          </PanelSection>
          <PanelSection title="Предобработка">
            <SettingField
              v-for="def in preprocessSettings"
              :key="`${mode}-${def.key}`"
              :def="def"
              :model-value="preprocess[mode][def.key]!"
              @update:model-value="setValue(preprocess[mode], def.key, $event)"
            />
          </PanelSection>
          <PanelSection title="Оптимизация">
            <label class="flex items-center justify-between text-[13px] text-ink-2">
              SVGO
              <Switch v-model="optimizeOn" />
            </label>
            <SettingField
              :def="{ kind: 'slider', key: 'precision', label: 'Точность', min: 0, max: 3, step: 1, value: 2, unit: 'зн.' }"
              :model-value="precision"
              @update:model-value="precision = Number($event)"
            />
            <label class="flex items-center justify-between text-[13px] text-ink-2">
              Многопроход
              <Switch v-model="multipass" />
            </label>
            <label class="flex items-center justify-between text-[13px] text-ink-2">
              Убрать размеры
              <Switch v-model="removeDimensions" />
            </label>
          </PanelSection>
        </template>
      </ScrollArea>
    </div>
    <div v-if="loaded && (final || runError || dirty)" class="grid gap-3 border-t border-line p-4">
      <p v-if="runError" class="text-xs leading-relaxed text-danger">
        {{ runError }}
      </p>
      <Button v-if="dirty" variant="ghost" class="h-9 rounded-full bg-raised shadow-raise hover:bg-surface" @click="run">
        Обновить
      </Button>
      <template v-if="final">
        <Stats />
        <ExportActions />
      </template>
    </div>
  </aside>
</template>
