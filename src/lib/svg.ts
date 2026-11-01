/** Ссылки `url(#id)`, у которых нет элемента с таким `id`. */
export function brokenReferences(svg: string) {
  const ids = new Set([...svg.matchAll(/\sid="([^"]+)"/g)].map(([, id]) => id!))
  const references = [...svg.matchAll(/url\(#([^)]+)\)/g)].map(([, id]) => id!)
  return references.filter(id => !ids.has(id))
}

/** Подмена цветов прямо в сыром SVG: новый прогон для этого не нужен. */
export function applyColors(svg: string, overrides: Record<string, string>) {
  return Object.entries(overrides).reduce(
    (result, [from, to]) => result.replaceAll(`fill="${from}"`, `fill="${to}"`),
    svg,
  )
}

/** Вид «контуры»: заливка убрана, толщина не зависит от масштаба. */
export function asOutlines(svg: string) {
  const style = '<style>*{fill:none!important;stroke:#ff2d78;stroke-width:1px;vector-effect:non-scaling-stroke}</style>'
  return svg
    .replace(/(<svg\b[^>]*>)/, `$1${style}`)
    .replace(/\sclip-path="[^"]*"/g, '')
}
