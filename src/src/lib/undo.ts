import type { TransferResult } from './operationState'

export type UndoEntry =
  | { kind: 'move'; results: TransferResult[] }
  | { kind: 'copy'; results: TransferResult[] }
  | { kind: 'rename'; renames: Array<{ from: string; to: string }> }
  | { kind: 'trash'; ids: string[]; count: number }
  | { kind: 'create'; path: string }

/** What undoing an entry will do, in the words the confirmation and the menu use. */
export type UndoPlan =
  | { action: 'move'; items: Array<{ path: string; destination: string }>; label: string }
  | { action: 'trash'; paths: string[]; label: string }
  | { action: 'rename'; renames: Array<{ path: string; to: string }>; label: string }
  | { action: 'restore'; ids: string[]; label: string }

export const UNDO_STACK_LIMIT = 50

export function parentOf(path: string): string {
  const cut = path.lastIndexOf('/')

  return cut <= 0 ? '/' : path.slice(0, cut)
}

export function basenameOf(path: string): string {
  return path.split('/').filter(Boolean).at(-1) ?? path
}

function countLabel(count: number, one: string, many: string): string {
  return count === 1 ? one : `${count} ${many}`
}

/**
 * Turns a recorded entry into the operations that reverse it. Returns null when
 * nothing survives to reverse — every item was skipped, or the entry is empty.
 */
export function planUndo(entry: UndoEntry): UndoPlan | null {
  switch (entry.kind) {
    case 'move': {
      const items = entry.results
        .filter((result) => !result.skipped)
        .map((result) => ({ path: result.destinationPath, destination: parentOf(result.sourcePath) }))

      return items.length === 0 ? null : { action: 'move', items, label: `Undo move of ${countLabel(items.length, 'one item', 'items')}` }
    }
    case 'copy': {
      const paths = entry.results.filter((result) => !result.skipped).map((result) => result.destinationPath)

      return paths.length === 0 ? null : { action: 'trash', paths, label: `Undo copy of ${countLabel(paths.length, 'one item', 'items')}` }
    }
    case 'rename': {
      const renames = entry.renames
        .filter((rename) => rename.from !== rename.to)
        .map((rename) => ({ path: rename.to, to: basenameOf(rename.from) }))

      return renames.length === 0 ? null : { action: 'rename', renames, label: `Undo rename of ${countLabel(renames.length, 'one item', 'items')}` }
    }
    case 'trash':
      return entry.ids.length === 0 ? null : { action: 'restore', ids: entry.ids, label: `Restore ${countLabel(entry.count, 'one item', 'items')}` }
    case 'create':
      return { action: 'trash', paths: [entry.path], label: `Undo new folder “${basenameOf(entry.path)}”` }
  }
}

/** Keeps the newest entries and drops the oldest past the limit. */
export function pushEntry(stack: UndoEntry[], entry: UndoEntry): UndoEntry[] {
  return [...stack, entry].slice(-UNDO_STACK_LIMIT)
}
