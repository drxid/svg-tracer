import type { Update } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import { check } from '@tauri-apps/plugin-updater'
import { ref } from 'vue'
import { toast } from 'vue-sonner'
import { errorMessage } from '@/lib/tauri'

/** Даём приложению открыться: проверка не должна конкурировать с первым прогоном. */
const CHECK_DELAY_MS = 3000

const readyVersion = ref<string | null>(null)
const installing = ref(false)
let pending: Update | null = null

/** Обновление скачивается молча, а ставится по клику: установка закрывает окно. */
async function checkForUpdate() {
  try {
    const update = await check()
    if (!update)
      return
    await update.download()
    pending = update
    readyVersion.value = update.version
  }
  catch (error) {
    // Нет сети или релиза: работе это не мешает, беспокоить незачем.
    console.warn('Проверка обновлений:', error)
  }
}

function startUpdateCheck() {
  if (!import.meta.env.DEV)
    setTimeout(() => void checkForUpdate(), CHECK_DELAY_MS)
}

async function installUpdate() {
  if (!pending || installing.value)
    return
  installing.value = true
  try {
    await pending.install()
    await relaunch()
  }
  catch (error) {
    installing.value = false
    toast.error(`Не удалось установить обновление: ${errorMessage(error)}`)
  }
}

export function useUpdater() {
  return { readyVersion, installing, startUpdateCheck, installUpdate }
}
