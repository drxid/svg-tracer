import type { UnlistenFn } from '@tauri-apps/api/event'
import type { ImageInfo, ModuleState, TraceOutcome, TraceSettings } from './types'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import { open, save } from '@tauri-apps/plugin-dialog'
import { LazyStore } from '@tauri-apps/plugin-store'

const IMAGE_FILTER = { name: 'Изображения', extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'gif'] }

export const store = new LazyStore('settings.json')

export const loadImagePath = (path: string) => invoke<ImageInfo>('load_image_path', { path })
export const loadImageBytes = (bytes: ArrayBuffer) => invoke<ImageInfo>('load_image_bytes', bytes)
export const getPreview = (imageId: number) => invoke<ArrayBuffer>('get_preview', { imageId })
export const cancelTrace = () => invoke<void>('cancel_trace')
export const saveFile = (path: string, contents: string) => invoke<void>('save_file', { path, contents })
export const modulesState = () => invoke<ModuleState>('modules_state')
export const modulesRetry = () => invoke<void>('modules_retry')
export const copyText = (text: string) => writeText(text)

export function trace(imageId: number, runId: number, settings: TraceSettings) {
  return invoke<TraceOutcome>('trace', { imageId, runId, settings })
}

export function onModulesState(handler: (state: ModuleState) => void): Promise<UnlistenFn> {
  return listen<ModuleState>('modules://state', event => handler(event.payload))
}

export async function pickImage() {
  const path = await open({ multiple: false, filters: [IMAGE_FILTER] })
  return typeof path === 'string' ? path : null
}

export function pickSavePath(defaultPath: string, extension: string, label: string) {
  return save({ defaultPath, filters: [{ name: label, extensions: [extension] }] })
}

export interface DropHandlers {
  over: () => void
  leave: () => void
  drop: (paths: string[]) => void
}

export function onFileDrop({ over, leave, drop }: DropHandlers): Promise<UnlistenFn> {
  return getCurrentWebview().onDragDropEvent(({ payload }) => {
    if (payload.type === 'drop')
      drop(payload.paths)
    else if (payload.type === 'leave')
      leave()
    else
      over()
  })
}

export const appWindow = {
  minimize: () => getCurrentWindow().minimize(),
  toggleMaximize: () => getCurrentWindow().toggleMaximize(),
  close: () => getCurrentWindow().close(),
}

/** Сообщение ошибки из Rust приходит объектом `{ code, message }`. */
export function errorMessage(error: unknown) {
  if (error && typeof error === 'object' && 'message' in error)
    return String(error.message)
  return String(error)
}
