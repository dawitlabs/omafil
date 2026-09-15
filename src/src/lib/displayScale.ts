import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'

const NARROW = 820
const MID = 1000

/// Below this the difference is rounding, not a scaling fault worth correcting.
const DRIFT_THRESHOLD = 1.05

/**
 * omafil runs under XWayland, because WebKitGTK delivers pointer events at the
 * wrong coordinates under fractional Wayland scaling. XWayland in turn reports
 * a scale of 1 through every API the app can reach, so on a fractionally scaled
 * display everything would draw at physical-pixel size — far smaller than the
 * rest of the desktop and too small to hit.
 *
 * The compositor still knows the real scale, so that is where it comes from.
 * The correction is the CSS `zoom` property, which feeds back into layout, so
 * hit testing stays where things are drawn.
 *
 * Breakpoints cannot come from media queries either: they would read the
 * uncorrected viewport. The corrected width goes on the root element instead.
 */
/** What the window covers in the units the rest of the desktop is drawn in. */
async function desktopWidth(): Promise<number> {
  try {
    const current = getCurrentWindow()
    const [size, windowScale, compositorScale] = await Promise.all([
      current.innerSize(),
      current.scaleFactor(),
      invoke<number | null>('display_scale').catch(() => null),
    ])

    const scale = compositorScale && compositorScale > 0 ? compositorScale : windowScale
    const width = scale > 0 ? size.width / scale : 0

    return width > 0 ? width : window.innerWidth
  } catch {
    // Not running under Tauri: the viewport is the honest width.
    return window.innerWidth
  }
}

async function correct(): Promise<void> {
  const width = await desktopWidth()
  if (width <= 0) return

  const root = document.documentElement
  const drift = window.innerWidth / width

  root.style.zoom = drift > DRIFT_THRESHOLD ? String(drift) : ''
  root.dataset.width = width <= NARROW ? 'narrow' : width <= MID ? 'mid' : 'wide'
}

/**
 * The viewport in the units laid-out elements actually use. `window.innerWidth`
 * reports the uncorrected space, so anything sizing itself against the viewport
 * has to divide the zoom back out.
 */
export function viewport(): { width: number; height: number } {
  const zoom = Number.parseFloat(getComputedStyle(document.documentElement).zoom) || 1

  return { width: window.innerWidth / zoom, height: window.innerHeight / zoom }
}

export async function matchDisplayScale(): Promise<void> {
  await correct().catch(() => undefined)

  const recorrect = () => void correct().catch(() => undefined)
  window.addEventListener('resize', recorrect)
  void getCurrentWindow().onScaleChanged(recorrect).catch(() => undefined)
}
