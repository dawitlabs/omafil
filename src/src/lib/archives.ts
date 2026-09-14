/** Kept in step with EXTRACTABLE_EXTENSIONS in src-tauri/src/archive.rs. */
const EXTRACTABLE_EXTENSIONS = [
  'zip', 'tar', 'gz', 'tgz', 'bz2', 'tbz', 'tbz2', 'xz', 'txz', 'zst', 'tzst', 'lz4', 'lzma',
  '7z', 'iso', 'cab', 'rar',
]

export function isExtractable(name: string): boolean {
  const extension = name.toLowerCase().split('.').at(-1)

  return name.includes('.') && extension !== undefined && EXTRACTABLE_EXTENSIONS.includes(extension)
}
