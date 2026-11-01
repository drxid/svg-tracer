export type Mode = 'color' | 'outline' | 'logo'
export type CompareView = 'split' | 'result' | 'source'
export type CanvasBackground = 'checker' | 'white' | 'black'
export type ExportFormat = 'svg' | 'vue' | 'react' | 'dataUri'

export type ModuleState
  = | { status: 'checking' }
    | { status: 'ready' }
    | { status: 'downloading', percent: number }
    | { status: 'failed', message: string }

export interface SliderDef {
  kind: 'slider'
  key: string
  label: string
  min: number
  max: number
  step: number
  value: number
  unit?: string
  /** Ноль означает «выключено». */
  zeroOff?: boolean
}

export interface ChoiceDef {
  kind: 'choice'
  key: string
  label: string
  options: { value: string, label: string }[]
  value: string
}

export interface ColorDef {
  kind: 'color'
  key: string
  label: string
  value: string
}

export type SettingDef = SliderDef | ChoiceDef | ColorDef
export type SettingValues = Record<string, number | string>

export interface Preset {
  id: string
  name: string
  hint: string
  mode: Mode
  values?: SettingValues
  preprocess?: SettingValues
  custom?: boolean
}

export interface Rect {
  x: number
  y: number
  width: number
  height: number
}

export interface Suggestion {
  mode: Mode
  colors: number
}

export interface ImageInfo {
  imageId: number
  name: string
  width: number
  height: number
  hasAlpha: boolean
  bytes: number
  suggested: Suggestion
}

export interface TraceOutcome {
  runId: number
  cancelled: boolean
  svg?: string
  bounds?: Rect
  ms?: number
  palette?: string[]
}

export interface TraceSettings {
  mode: Mode
  preprocess: {
    crop: Rect | null
    scale: number
    background: string
    backgroundDistance: number
    cropToArtwork: boolean
    padding: number
  }
  color: {
    hierarchical: string
    lines: string
    speckle: number
    colorPrecision: number
    gradientStep: number
    simplify: number
    maxColors: number
  }
  outline: {
    threshold: number
    invert: boolean
    speckle: number
    alphamax: number
    tolerance: number
    fill: string
  }
  logo: {
    colors: number
    speckle: number
    alphamax: number
    tolerance: number
    polygon: number
    fill: string
  }
}
