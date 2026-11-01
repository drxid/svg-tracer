import type { CanvasBackground, CompareView, ExportFormat, ImageInfo, Mode, ModuleState, Preset, Rect, SettingValues } from '@/lib/types'
import type { OptimizeRequest, OptimizeResponse } from '@/workers/svgo.worker'
import { computed, reactive, ref, shallowRef, watch } from 'vue'
import { toast } from 'vue-sonner'
import { exportAs } from '@/lib/export'
import { defaultPreprocess, defaultValues, MODE_SETTINGS, MODES, PRESETS, toTraceSettings } from '@/lib/modes'
import { measure } from '@/lib/stats'
import { applyColors } from '@/lib/svg'
import * as tauri from '@/lib/tauri'

const DEBOUNCE_MS = 200
const SLOW_RUN_MS = 2000
const SLOW_RUNS_LIMIT = 2
const PILL_LINGER_MS = 1800
const HISTORY_LIMIT = 100
const CUSTOM_PRESETS_KEY = 'customPresets'

const image = shallowRef<ImageInfo | null>(null)
const sourceUrl = ref('')
const loaded = computed(() => image.value !== null)

const mode = ref<Mode>('logo')
const modeChosen = ref(false)
const activePreset = ref<string | null>(null)
const customPresets = ref<Preset[]>([])
const presets = computed(() => [...PRESETS, ...customPresets.value])
const values = reactive<Record<Mode, SettingValues>>({
  color: defaultValues(MODE_SETTINGS.color),
  outline: defaultValues(MODE_SETTINGS.outline),
  logo: defaultValues(MODE_SETTINGS.logo),
})
const preprocess = reactive<Record<Mode, SettingValues>>({
  color: defaultPreprocess('color'),
  outline: defaultPreprocess('outline'),
  logo: defaultPreprocess('logo'),
})
const crop = ref<Rect | null>(null)

const optimizeOn = ref(true)
const precision = ref(2)
const multipass = ref(true)
const removeDimensions = ref(true)

const rawSvg = ref('')
const bounds = ref<Rect | null>(null)
const palette = ref<string[]>([])
const colorOverrides = ref<Record<string, string>>({})
const durationMs = ref(0)
const final = ref('')
const optimizeWarning = ref('')
const running = ref(false)
const runError = ref('')
const liveOff = ref(false)
const dirty = ref(false)

const view = ref<CompareView>('split')
const canvasBackground = ref<CanvasBackground>('checker')
const showOutlines = ref(false)
const cropping = ref(false)

const potrace = ref<ModuleState>({ status: 'checking' })
const potraceSettled = ref(true)
const usesPotrace = computed(() => mode.value === 'outline' || (mode.value === 'logo' && Number(values.logo.polygon) === 0))
const modeLocked = computed(() => usesPotrace.value && potrace.value.status !== 'ready')

const editedSvg = computed(() => applyColors(rawSvg.value, colorOverrides.value))
const rawStats = computed(() => measure(editedSvg.value))
const finalStats = computed(() => measure(final.value))
const resultUrl = ref('')

let runId = 0
let slowRuns = 0
let debounceTimer: ReturnType<typeof setTimeout> | undefined

// --- SVGO: держим одно задание в работе и одно ожидающее.

const worker = new Worker(new URL('../workers/svgo.worker.ts', import.meta.url), { type: 'module' })
let optimizeId = 0
let optimizeBusy = false
let optimizePending: OptimizeRequest | null = null

function postOptimize(request: OptimizeRequest) {
  optimizeBusy = true
  worker.postMessage(request)
}

worker.onmessage = ({ data }: MessageEvent<OptimizeResponse>) => {
  optimizeBusy = false
  if (data.id === optimizeId) {
    final.value = data.svg
    optimizeWarning.value = data.warning ?? ''
  }
  if (optimizePending) {
    postOptimize(optimizePending)
    optimizePending = null
  }
}

watch([editedSvg, optimizeOn, precision, multipass, removeDimensions], () => {
  optimizeId += 1
  optimizePending = null
  optimizeWarning.value = ''
  if (!editedSvg.value || !optimizeOn.value) {
    final.value = editedSvg.value
    return
  }
  const request: OptimizeRequest = {
    id: optimizeId,
    svg: editedSvg.value,
    precision: precision.value,
    multipass: multipass.value,
    removeDimensions: removeDimensions.value,
  }
  if (optimizeBusy)
    optimizePending = request
  else
    postOptimize(request)
})

watch(final, (svg) => {
  if (resultUrl.value)
    URL.revokeObjectURL(resultUrl.value)
  resultUrl.value = svg ? URL.createObjectURL(new Blob([svg], { type: 'image/svg+xml' })) : ''
})

// --- История настроек.

interface Snapshot {
  mode: Mode
  activePreset: string | null
  values: Record<Mode, SettingValues>
  preprocess: Record<Mode, SettingValues>
  crop: Rect | null
}

const history: string[] = []
let historyIndex = -1

function snapshot() {
  const state: Snapshot = { mode: mode.value, activePreset: activePreset.value, values, preprocess, crop: crop.value }
  return JSON.stringify(state)
}

function commitHistory() {
  const state = snapshot()
  if (history[historyIndex] === state)
    return
  history.length = historyIndex + 1
  history.push(state)
  if (history.length > HISTORY_LIMIT)
    history.shift()
  historyIndex = history.length - 1
}

function restore(index: number) {
  const state = JSON.parse(history[index]!) as Snapshot
  historyIndex = index
  mode.value = state.mode
  activePreset.value = state.activePreset
  for (const name of MODES.map(item => item.value)) {
    Object.assign(values[name], state.values[name])
    Object.assign(preprocess[name], state.preprocess[name])
  }
  crop.value = state.crop
  schedule({ remember: false, force: true })
}

function undo() {
  if (historyIndex > 0)
    restore(historyIndex - 1)
}

function redo() {
  if (historyIndex < history.length - 1)
    restore(historyIndex + 1)
}

// --- Прогоны.

async function run() {
  const current = image.value
  if (!current || modeLocked.value)
    return
  clearTimeout(debounceTimer)
  runId += 1
  const id = runId
  running.value = true
  dirty.value = false
  try {
    const settings = toTraceSettings(mode.value, values, preprocess[mode.value], crop.value)
    const result = await tauri.trace(current.imageId, id, settings)
    if (id !== runId || result.cancelled)
      return
    rawSvg.value = result.svg!
    bounds.value = result.bounds!
    palette.value = result.palette ?? []
    durationMs.value = result.ms!
    runError.value = ''
    // Первый прогон считает все стадии с нуля, он не показатель: подбор
    // настроек идёт по кэшу и обычно намного быстрее.
    slowRuns = result.ms! > SLOW_RUN_MS ? slowRuns + 1 : 0
    liveOff.value = slowRuns >= SLOW_RUNS_LIMIT
  }
  catch (error) {
    if (id === runId)
      runError.value = tauri.errorMessage(error)
  }
  finally {
    if (id === runId)
      running.value = false
  }
}

/** Слайдер при выключенном живом обновлении только помечает результат устаревшим. */
function schedule({ remember = true, force = false } = {}) {
  clearTimeout(debounceTimer)
  debounceTimer = setTimeout(() => {
    if (remember)
      commitHistory()
    if (!image.value)
      return
    if (liveOff.value && !force)
      dirty.value = true
    else
      run()
  }, DEBOUNCE_MS)
}

function cancelRun() {
  clearTimeout(debounceTimer)
  runId += 1
  running.value = false
  void tauri.cancelTrace()
}

// --- Настройки.

function setMode(next: Mode) {
  if (next === mode.value)
    return
  mode.value = next
  modeChosen.value = true
  activePreset.value = null
  void tauri.store.set('mode', next)
  schedule({ force: true })
}

function setValue(target: SettingValues, key: string, value: number | string) {
  target[key] = value
  activePreset.value = null
  schedule()
}

function applyPreset(preset: Preset) {
  mode.value = preset.mode
  modeChosen.value = true
  Object.assign(values[preset.mode], defaultValues(MODE_SETTINGS[preset.mode]), preset.values)
  Object.assign(preprocess[preset.mode], defaultPreprocess(preset.mode), preset.preprocess)
  activePreset.value = preset.id
  schedule({ force: true })
}

function saveCustomPresets() {
  void tauri.store.set(CUSTOM_PRESETS_KEY, customPresets.value)
}

function addPreset(name: string) {
  const preset: Preset = {
    id: `custom-${Date.now()}`,
    name,
    hint: MODES.find(item => item.value === mode.value)!.label,
    mode: mode.value,
    values: { ...values[mode.value] },
    preprocess: { ...preprocess[mode.value] },
    custom: true,
  }
  customPresets.value = [...customPresets.value, preset]
  activePreset.value = preset.id
  saveCustomPresets()
}

function removePreset(id: string) {
  customPresets.value = customPresets.value.filter(preset => preset.id !== id)
  if (activePreset.value === id)
    activePreset.value = null
  saveCustomPresets()
}

function setCrop(next: Rect | null) {
  crop.value = next
  cropping.value = false
  schedule({ force: true })
}

function recolor(from: string, to: string) {
  colorOverrides.value = { ...colorOverrides.value, [from]: to }
}

// --- Загрузка.

async function adopt(load: Promise<ImageInfo>) {
  try {
    const info = await load
    const preview = await tauri.getPreview(info.imageId)
    runId += 1
    if (sourceUrl.value)
      URL.revokeObjectURL(sourceUrl.value)
    sourceUrl.value = URL.createObjectURL(new Blob([preview]))
    image.value = info
    crop.value = null
    cropping.value = false
    rawSvg.value = ''
    bounds.value = null
    palette.value = []
    colorOverrides.value = {}
    runError.value = ''
    liveOff.value = false
    slowRuns = 0
    if (!modeChosen.value) {
      mode.value = info.suggested.mode
      activePreset.value = null
      if (info.suggested.mode === 'logo')
        values.logo.colors = info.suggested.colors
    }
    await run()
  }
  catch (error) {
    toast.error(tauri.errorMessage(error))
  }
}

function openPath(path: string) {
  return adopt(tauri.loadImagePath(path))
}

function openBytes(bytes: ArrayBuffer) {
  return adopt(tauri.loadImageBytes(bytes))
}

async function openDialog() {
  const path = await tauri.pickImage()
  if (path)
    await openPath(path)
}

function openDropped(paths: string[]) {
  if (paths.length > 1)
    toast('Открыт только первый файл, остальные пропущены')
  if (paths[0])
    void openPath(paths[0])
}

// --- Экспорт.

const FORMAT_LABELS: Record<ExportFormat, string> = {
  svg: 'SVG',
  vue: 'Компонент Vue',
  react: 'Компонент React',
  dataUri: 'Data URI',
}

const SAVE_TARGETS: Partial<Record<ExportFormat, { extension: string, label: string }>> = {
  svg: { extension: 'svg', label: 'SVG' },
  vue: { extension: 'vue', label: 'Компонент Vue' },
  react: { extension: 'tsx', label: 'Компонент React' },
}

async function copy(format: ExportFormat) {
  if (!final.value)
    return
  await tauri.copyText(exportAs(format, final.value, image.value!.name))
  toast(`${FORMAT_LABELS[format]}: скопировано`)
}

async function save(format: ExportFormat = 'svg') {
  const target = SAVE_TARGETS[format]
  if (!final.value || !target)
    return
  const name = image.value!.name.replace(/\.[^.]+$/, '')
  const path = await tauri.pickSavePath(`${name}.${target.extension}`, target.extension, target.label)
  if (!path)
    return
  try {
    await tauri.saveFile(path, exportAs(format, final.value, image.value!.name))
    toast('Сохранено')
  }
  catch (error) {
    toast.error(tauri.errorMessage(error))
  }
}

// --- Модули.

function setPotrace(state: ModuleState) {
  const wasReady = potrace.value.status === 'ready'
  potrace.value = state
  if (state.status === 'downloading' || state.status === 'failed') {
    potraceSettled.value = false
  }
  else if (state.status === 'ready' && !potraceSettled.value) {
    setTimeout(() => potraceSettled.value = true, PILL_LINGER_MS)
  }
  if (state.status === 'ready' && !wasReady && image.value)
    void run()
}

async function startup() {
  await tauri.onModulesState(setPotrace)
  setPotrace(await tauri.modulesState())
  const savedMode = await tauri.store.get<Mode>('mode')
  if (savedMode)
    mode.value = savedMode
  customPresets.value = await tauri.store.get<Preset[]>(CUSTOM_PRESETS_KEY) ?? []
  commitHistory()
}

function retryModules() {
  void tauri.modulesRetry()
}

export function useApp() {
  return {
    image,
    sourceUrl,
    loaded,
    mode,
    activePreset,
    presets,
    values,
    preprocess,
    crop,
    optimizeOn,
    precision,
    multipass,
    removeDimensions,
    final,
    bounds,
    palette,
    colorOverrides,
    durationMs,
    optimizeWarning,
    running,
    runError,
    liveOff,
    dirty,
    view,
    canvasBackground,
    showOutlines,
    cropping,
    potrace,
    potraceSettled,
    modeLocked,
    rawStats,
    finalStats,
    resultUrl,
    run,
    cancelRun,
    setMode,
    setValue,
    applyPreset,
    addPreset,
    removePreset,
    setCrop,
    recolor,
    undo,
    redo,
    copy,
    save,
    openDialog,
    openBytes,
    openDropped,
    retryModules,
    startup,
  }
}
