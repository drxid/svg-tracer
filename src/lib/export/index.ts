import type { ExportFormat } from '../types'

const REACT_ATTRIBUTES: Record<string, string> = {
  'fill-rule': 'fillRule',
  'clip-rule': 'clipRule',
  'clip-path': 'clipPath',
  'stroke-width': 'strokeWidth',
  'stroke-linecap': 'strokeLinecap',
  'stroke-linejoin': 'strokeLinejoin',
}

function toComponentName(name: string) {
  const parts = name.replace(/\.[^.]+$/, '').split(/[^a-z0-9]+/i).filter(Boolean)
  const joined = parts.map(part => part[0]!.toUpperCase() + part.slice(1)).join('')
  return /^[A-Z]/.test(joined) ? joined : `Icon${joined}`
}

function withRootProps(svg: string, props: string) {
  return svg.replace(/<svg\b/, `<svg ${props}`)
}

export function toVue(svg: string) {
  return `<template>\n  ${withRootProps(svg.trim(), 'v-bind="$attrs"')}\n</template>\n`
}

export function toReact(svg: string, fileName: string) {
  const name = toComponentName(fileName)
  const jsx = Object.entries(REACT_ATTRIBUTES).reduce(
    (result, [from, to]) => result.replaceAll(`${from}=`, `${to}=`),
    svg.trim(),
  )
  return [
    `import type { SVGProps } from 'react'`,
    '',
    `export function ${name}(props: SVGProps<SVGSVGElement>) {`,
    `  return (\n    ${withRootProps(jsx, '{...props}')}\n  )`,
    '}',
    '',
  ].join('\n')
}

export function toDataUri(svg: string) {
  const encoded = svg.trim()
    .replace(/"/g, '\'')
    .replace(/%/g, '%25')
    .replace(/#/g, '%23')
    .replace(/</g, '%3C')
    .replace(/>/g, '%3E')
    .replace(/\s+/g, ' ')
  return `url("data:image/svg+xml,${encoded}")`
}

export function exportAs(format: ExportFormat, svg: string, fileName: string) {
  if (format === 'vue')
    return toVue(svg)
  if (format === 'react')
    return toReact(svg, fileName)
  if (format === 'dataUri')
    return toDataUri(svg)
  return svg
}
