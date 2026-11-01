import { optimize } from 'svgo/browser'
import { brokenReferences } from '@/lib/svg'

export interface OptimizeOptions {
  precision: number
  multipass: boolean
  removeDimensions: boolean
}

export interface OptimizeRequest extends OptimizeOptions {
  id: number
  svg: string
}

export interface OptimizeResponse {
  id: number
  svg: string
  warning?: string
}

const ROLLBACK = 'SVGO сломал ссылки в файле, показан результат без оптимизации'

function run({ svg, precision, multipass, removeDimensions }: OptimizeRequest) {
  const { data } = optimize(svg, {
    multipass,
    plugins: [
      {
        name: 'preset-default',
        params: {
          overrides: {
            // Без этого `clipPath` логотипа теряет id и слои исчезают.
            ...(svg.includes('<clipPath') ? { cleanupIds: false as const } : {}),
            cleanupNumericValues: { floatPrecision: precision },
            convertPathData: { floatPrecision: precision },
          },
        },
      },
      ...(removeDimensions ? ['removeDimensions' as const] : []),
    ],
  })
  return data
}

self.onmessage = ({ data }: MessageEvent<OptimizeRequest>) => {
  const reply = (response: Omit<OptimizeResponse, 'id'>) => self.postMessage({ id: data.id, ...response })
  try {
    const svg = run(data)
    if (brokenReferences(svg).length > 0)
      reply({ svg: data.svg, warning: ROLLBACK })
    else
      reply({ svg })
  }
  catch (error) {
    reply({ svg: data.svg, warning: `SVGO: ${String(error)}` })
  }
}
