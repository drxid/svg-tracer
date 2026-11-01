import { describe, expect, it } from 'vitest'
import { toDataUri, toReact, toVue } from './export'
import { defaultPreprocess, defaultValues, isVisible, MODE_SETTINGS, PREPROCESS_SETTINGS, toTraceSettings } from './modes'
import { measure } from './stats'
import { applyColors, asOutlines, brokenReferences } from './svg'

const LOGO = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8"><clipPath id="shape"><path clip-rule="evenodd" d="M0 0L8 0Z"/></clipPath><g clip-path="url(#shape)"><path fill="#ff0000" d="M1 1C2 2 3 3 4 4Z"/></g></svg>'

describe('ссылки', () => {
  it('целый файл', () => {
    expect(brokenReferences(LOGO)).toEqual([])
  })
  it('находит удалённый clipPath', () => {
    expect(brokenReferences(LOGO.replace(' id="shape"', ''))).toEqual(['shape'])
  })
})

describe('статистика', () => {
  it('считает пути, узлы и цвета', () => {
    const stats = measure(LOGO)
    expect(stats.paths).toBe(2)
    expect(stats.nodes).toBe(6)
    expect(stats.colors).toEqual(['#ff0000'])
  })
})

describe('правка цветов', () => {
  it('меняет только заливку', () => {
    expect(applyColors(LOGO, { '#ff0000': '#00ff00' })).toContain('fill="#00ff00"')
  })
  it('контуры снимают клип, иначе обводка обрезается', () => {
    const outlined = asOutlines(LOGO)
    expect(outlined).toContain('<style>')
    expect(outlined).not.toContain('clip-path="')
  })
})

describe('экспорт', () => {
  it('Vue пробрасывает атрибуты', () => {
    expect(toVue(LOGO)).toContain('<svg v-bind="$attrs"')
  })
  it('React переводит атрибуты в camelCase и даёт имя компоненту', () => {
    const code = toReact(LOGO, 'my-logo.final.png')
    expect(code).toContain('export function MyLogoFinal(')
    expect(code).toContain('clipPath="url(#shape)"')
    expect(code).toContain('clipRule=')
    expect(code).not.toContain('clip-path')
  })
  it('имя из цифр остаётся валидным идентификатором', () => {
    expect(toReact(LOGO, '123.png')).toContain('export function Icon123(')
  })
  it('data URI экранирован для CSS', () => {
    const uri = toDataUri(LOGO)
    expect(uri.startsWith('url("data:image/svg+xml,')).toBe(true)
    expect(uri.slice(24, -2)).not.toMatch(/[#<>"]/)
  })
})

describe('настройки', () => {
  const values = {
    color: defaultValues(MODE_SETTINGS.color),
    outline: defaultValues(MODE_SETTINGS.outline),
    logo: defaultValues(MODE_SETTINGS.logo),
  }

  it('у режимов свои значения предобработки', () => {
    expect(defaultPreprocess('logo').scale).toBe('4')
    expect(defaultPreprocess('outline').scale).toBe('2')
    expect(defaultPreprocess('color')).toMatchObject({ scale: '1', background: 'keep' })
  })
  it('цвет фона идёт в Rust как hex', () => {
    const preprocess = { ...defaultPreprocess('logo'), background: 'color', backgroundColor: '#123456' }
    expect(toTraceSettings('logo', values, preprocess, null).preprocess.background).toBe('#123456')
  })
  it('порог виден, только когда фон не удаляется', () => {
    const threshold = MODE_SETTINGS.outline.find(def => def.key === 'threshold')!
    expect(isVisible(threshold, values.outline, defaultPreprocess('outline'))).toBe(false)
    expect(isVisible(threshold, values.outline, { ...defaultPreprocess('outline'), background: 'keep' })).toBe(true)
  })
  it('цвет заливки логотипа виден только при одном цвете', () => {
    const fill = MODE_SETTINGS.logo.find(def => def.key === 'fill')!
    expect(isVisible(fill, { ...values.logo, colors: 4 }, defaultPreprocess('logo'))).toBe(false)
    expect(isVisible(fill, { ...values.logo, colors: 1 }, defaultPreprocess('logo'))).toBe(true)
  })
  it('описания настроек не дублируют ключи', () => {
    const keys = PREPROCESS_SETTINGS.map(def => def.key)
    expect(new Set(keys).size).toBe(keys.length)
  })
})
