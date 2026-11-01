import { useDark, useToggle } from '@vueuse/core'

export function useTheme() {
  const isDark = useDark({ storageKey: 'svg-tracer-theme' })
  return { isDark, toggleTheme: useToggle(isDark) }
}
