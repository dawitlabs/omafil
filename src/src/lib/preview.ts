import { convertFileSrc, invoke } from '@tauri-apps/api/core'

/** Ask the backend to authorize this one regular media file, including outside home. */
export async function mediaPreviewSource(path: string): Promise<string> {
  return convertFileSrc(await invoke<string>('preview_asset', { path }))
}

export async function pdfPreviewSource(path: string): Promise<string> {
  return mediaPreviewSource(await invoke<string>('pdf_preview', { path }))
}

/** Anything with no preview of its own: the shared freedesktop thumbnail cache,
 * then an installed thumbnailer. Rejects when neither covers the file type. */
export async function thumbnailSource(path: string): Promise<string> {
  return mediaPreviewSource(await invoke<string>('thumbnail', { path }))
}
