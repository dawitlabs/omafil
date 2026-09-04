const BYTE_UNITS = ['B', 'KB', 'MB', 'GB', 'TB'] as const

const dateFormatter = new Intl.DateTimeFormat(undefined, {
  dateStyle: 'short',
  timeStyle: 'short',
})

export function formatBytes(bytes: number): string {
  let unitIndex = 0
  let value = bytes

  while (value >= 1024 && unitIndex < BYTE_UNITS.length - 1) {
    value /= 1024
    unitIndex += 1
  }

  return `${value >= 10 || unitIndex === 0 ? Math.round(value) : value.toFixed(1)} ${BYTE_UNITS[unitIndex]}`
}

export function formatModified(seconds: number | null): string {
  if (seconds === null) return ''

  return dateFormatter.format(new Date(seconds * 1000))
}

export function formatCount(count: number): string {
  return count.toLocaleString()
}

export function formatItems(count: number): string {
  return `${formatCount(count)} ${count === 1 ? 'item' : 'items'}`
}

export function parentFolder(path: string): string {
  const parent = path.slice(0, path.lastIndexOf('/'))

  return parent || '/'
}

export function typeLabel(name: string, isDirectory: boolean): string {
  if (isDirectory) return 'File folder'

  const [stem, extension] = [name.slice(0, name.lastIndexOf('.')), name.slice(name.lastIndexOf('.') + 1)]

  return stem && extension !== name ? `${extension.toUpperCase()} file` : 'File'
}
