import { invoke } from '@tauri-apps/api/core'

export type DictationStatus = { isAvailable: boolean; state: 'idle' | 'recording' | 'transcribing' }

export async function dictationStatus(): Promise<DictationStatus> {
  return invoke<DictationStatus>('dictation_status')
}

export async function toggleDictation(): Promise<void> {
  await invoke('toggle_dictation')
}
