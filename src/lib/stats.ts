export interface SvgStats {
  bytes: number
  paths: number
  nodes: number
  colors: string[]
}

export function measure(svg: string): SvgStats {
  const paths = [...svg.matchAll(/\sd="([^"]*)"/g)]
  const nodes = paths.reduce((sum, [, d]) => sum + (d?.match(/[a-z]/gi)?.length ?? 0), 0)
  const fills = [...svg.matchAll(/fill="(#[0-9a-f]{3,8}|rgb\([^)]*\))"/gi)].map(([, fill]) => fill!.toLowerCase())
  return {
    bytes: new Blob([svg]).size,
    paths: paths.length,
    nodes,
    colors: [...new Set(fills)],
  }
}

const number = new Intl.NumberFormat('ru', { maximumFractionDigits: 1 })

export function formatNumber(value: number) {
  return number.format(value)
}

export function formatBytes(bytes: number) {
  return `${number.format(bytes / 1024)} КБ`
}
