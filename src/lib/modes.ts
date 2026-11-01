import type { Mode, Preset, Rect, SettingDef, SettingValues, TraceSettings } from './types'

export const MODES: { value: Mode, label: string }[] = [
  { value: 'color', label: 'Цвет' },
  { value: 'outline', label: 'Контур' },
  { value: 'logo', label: 'Логотип' },
]

export const MODE_SETTINGS: Record<Mode, SettingDef[]> = {
  color: [
    { kind: 'choice', key: 'hierarchical', label: 'Сборка слоёв', value: 'stacked', options: [{ value: 'stacked', label: 'Стопка' }, { value: 'cutout', label: 'Мозаика' }] },
    { kind: 'choice', key: 'lines', label: 'Линии', value: 'spline', options: [{ value: 'spline', label: 'Кривые' }, { value: 'polygon', label: 'Прямые' }, { value: 'pixel', label: 'Пиксели' }] },
    { kind: 'slider', key: 'speckle', label: 'Мусор', min: 0, max: 128, step: 1, value: 4, unit: 'px' },
    { kind: 'slider', key: 'colorPrecision', label: 'Точность цвета', min: 1, max: 8, step: 1, value: 6, unit: 'бит' },
    { kind: 'slider', key: 'gradientStep', label: 'Градиенты', min: 0, max: 255, step: 1, value: 16 },
    { kind: 'slider', key: 'maxColors', label: 'Максимум цветов', min: 0, max: 64, step: 1, value: 0, zeroOff: true },
    { kind: 'slider', key: 'simplify', label: 'Упрощение', min: 0, max: 5, step: 0.1, value: 1, unit: 'px' },
  ],
  outline: [
    { kind: 'slider', key: 'threshold', label: 'Порог', min: 0, max: 255, step: 1, value: 128 },
    { kind: 'choice', key: 'invert', label: 'Инвертировать', value: 'no', options: [{ value: 'no', label: 'Нет' }, { value: 'yes', label: 'Да' }] },
    { kind: 'slider', key: 'speckle', label: 'Мусор', min: 0, max: 100, step: 1, value: 4, unit: 'px' },
    { kind: 'slider', key: 'alphamax', label: 'Углы', min: 0, max: 1.34, step: 0.01, value: 1 },
    { kind: 'slider', key: 'tolerance', label: 'Упрощение кривых', min: 0, max: 1, step: 0.01, value: 0.4 },
    { kind: 'choice', key: 'fill', label: 'Цвет', value: 'source', options: [{ value: 'source', label: 'Из рисунка' }, { value: 'currentColor', label: 'currentColor' }] },
  ],
  logo: [
    { kind: 'slider', key: 'colors', label: 'Цвета', min: 1, max: 16, step: 1, value: 4 },
    { kind: 'slider', key: 'speckle', label: 'Мусор', min: 0, max: 100, step: 1, value: 4, unit: 'px' },
    { kind: 'slider', key: 'alphamax', label: 'Углы', min: 0, max: 1.34, step: 0.01, value: 1 },
    { kind: 'slider', key: 'tolerance', label: 'Упрощение кривых', min: 0, max: 1, step: 0.01, value: 0.4 },
    { kind: 'slider', key: 'polygon', label: 'Прямые линии', min: 0, max: 5, step: 0.1, value: 0, unit: 'px', zeroOff: true },
    { kind: 'choice', key: 'fill', label: 'Цвет', value: 'source', options: [{ value: 'source', label: 'Из рисунка' }, { value: 'currentColor', label: 'currentColor' }] },
  ],
}

export const PREPROCESS_SETTINGS: SettingDef[] = [
  { kind: 'choice', key: 'scale', label: 'Увеличение', value: '4', options: ['1', '2', '3', '4'].map(value => ({ value, label: `×${value}` })) },
  { kind: 'choice', key: 'background', label: 'Фон', value: 'auto', options: [{ value: 'auto', label: 'Авто' }, { value: 'color', label: 'Цвет' }, { value: 'keep', label: 'Оставить' }] },
  { kind: 'color', key: 'backgroundColor', label: 'Цвет фона', value: '#ffffff' },
  { kind: 'slider', key: 'backgroundDistance', label: 'Чувствительность фона', min: 5, max: 120, step: 1, value: 20 },
  { kind: 'choice', key: 'cropToArtwork', label: 'Обрезать по рисунку', value: 'yes', options: [{ value: 'yes', label: 'Да' }, { value: 'no', label: 'Нет' }] },
  { kind: 'slider', key: 'padding', label: 'Поля', min: 0, max: 64, step: 1, value: 0, unit: 'px' },
]

/** В «Цвете» фон не трогаем: VTracer справляется сам, а авто ошибается на иллюстрациях. */
const PREPROCESS_BY_MODE: Record<Mode, SettingValues> = {
  color: { scale: '1', background: 'keep', cropToArtwork: 'no' },
  outline: { scale: '2' },
  logo: { scale: '4' },
}

export const PRESETS: Preset[] = [
  { id: 'logo', name: 'Логотип', hint: 'Плоские цвета', mode: 'logo' },
  { id: 'icon', name: 'Иконка', hint: 'currentColor', mode: 'outline', values: { fill: 'currentColor' }, preprocess: { scale: '4' } },
  { id: 'illustration', name: 'Иллюстрация', hint: 'Мозаика', mode: 'color', values: { hierarchical: 'cutout' } },
  { id: 'scan', name: 'Скан', hint: 'Чёрно-белый', mode: 'outline', values: { speckle: 16 }, preprocess: { background: 'keep', cropToArtwork: 'no' } },
  { id: 'geometry', name: 'Геометрия', hint: 'Прямые линии', mode: 'logo', values: { polygon: 1.5 } },
  { id: 'poster', name: 'Постер', hint: '16 цветов', mode: 'color', values: { maxColors: 16, simplify: 1.5 } },
  { id: 'pixel', name: 'Пиксель-арт', hint: 'Без сглаживания', mode: 'color', values: { lines: 'pixel', speckle: 0 } },
  { id: 'photo', name: 'Фото', hint: 'Тяжёлый файл', mode: 'color', values: { speckle: 10, colorPrecision: 8, gradientStep: 48, simplify: 2 } },
]

export function defaultValues(defs: SettingDef[]): SettingValues {
  return Object.fromEntries(defs.map(def => [def.key, def.value]))
}

export function defaultPreprocess(mode: Mode): SettingValues {
  return { ...defaultValues(PREPROCESS_SETTINGS), ...PREPROCESS_BY_MODE[mode] }
}

export function isVisible(def: SettingDef, values: SettingValues, preprocess: SettingValues) {
  if (def.key === 'threshold')
    return preprocess.background === 'keep'
  if (def.key === 'backgroundColor')
    return preprocess.background === 'color'
  if (def.key === 'backgroundDistance')
    return preprocess.background !== 'keep'
  if (def.key === 'fill' && 'colors' in values)
    return values.colors === 1
  return true
}

function backgroundOf(preprocess: SettingValues) {
  return preprocess.background === 'color' ? String(preprocess.backgroundColor) : String(preprocess.background)
}

export function toTraceSettings(
  mode: Mode,
  values: Record<Mode, SettingValues>,
  preprocess: SettingValues,
  crop: Rect | null,
): TraceSettings {
  const { color, outline, logo } = values
  return {
    mode,
    preprocess: {
      crop,
      scale: Number(preprocess.scale),
      background: backgroundOf(preprocess),
      backgroundDistance: Number(preprocess.backgroundDistance),
      cropToArtwork: preprocess.cropToArtwork === 'yes',
      padding: Number(preprocess.padding),
    },
    color: {
      hierarchical: String(color.hierarchical),
      lines: String(color.lines),
      speckle: Number(color.speckle),
      colorPrecision: Number(color.colorPrecision),
      gradientStep: Number(color.gradientStep),
      simplify: Number(color.simplify),
      maxColors: Number(color.maxColors),
    },
    outline: {
      threshold: Number(outline.threshold),
      invert: outline.invert === 'yes',
      speckle: Number(outline.speckle),
      alphamax: Number(outline.alphamax),
      tolerance: Number(outline.tolerance),
      fill: String(outline.fill),
    },
    logo: {
      colors: Number(logo.colors),
      speckle: Number(logo.speckle),
      alphamax: Number(logo.alphamax),
      tolerance: Number(logo.tolerance),
      polygon: Number(logo.polygon),
      fill: String(logo.fill),
    },
  }
}
