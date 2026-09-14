import { appState } from './appState.svelte'
import type { ContextMenuItem } from './components/ContextMenu/ContextMenu.svelte'
import { fileOperations } from './fileOperations.svelte'
import { tabs } from './tabs.svelte'

type FolderMenuOptions = {
  /** Starts an inline rename where the folder is shown, when that surface has one. */
  onRename?: () => void
  /** Reloads the surface the folder is shown on, after it gains or loses children. */
  onChanged?: () => void
}

/**
 * The operations that apply to a folder addressed by path alone, for surfaces
 * that have no selection of their own — the sidebar tree, pins and known locations.
 */
export function folderMenuItems(path: string, { onRename, onChanged }: FolderMenuOptions = {}): ContextMenuItem[] {
  const active = tabs.active

  return [
    { kind: 'action', label: 'Open', onSelect: () => active.open(path) },
    { kind: 'action', label: 'Open in new tab', onSelect: () => { tabs.open(); tabs.active.open(path) } },
    { kind: 'action', label: 'Open in terminal', shortcut: 'F4', onSelect: () => active.openTerminal(path) },
    { kind: 'action', label: 'Open externally', onSelect: () => active.openExternally(path) },
    { kind: 'separator' },
    { kind: 'action', label: 'New folder inside', shortcut: 'Ctrl+Shift+N', onSelect: async () => { await fileOperations.createFolderIn(path); onChanged?.() } },
    { kind: 'separator' },
    { kind: 'action', label: 'Cut', shortcut: 'Ctrl+X', onSelect: () => fileOperations.cutPaths([path]) },
    { kind: 'action', label: 'Copy', shortcut: 'Ctrl+C', onSelect: () => fileOperations.copyPaths([path]) },
    { kind: 'action', label: 'Paste into folder', shortcut: 'Ctrl+V', disabled: !fileOperations.canPaste, onSelect: () => fileOperations.pasteTo(path) },
    { kind: 'action', label: 'Copy path', onSelect: () => navigator.clipboard.writeText(path) },
    { kind: 'separator' },
    { kind: 'action', label: 'Compress to ZIP', onSelect: () => fileOperations.compressPaths([path], parentOf(path)) },
    { kind: 'separator' },
    ...(onRename ? [{ kind: 'action', label: 'Rename', shortcut: 'F2', onSelect: onRename } as ContextMenuItem] : []),
    { kind: 'action', label: 'Move to trash', shortcut: 'Del', onSelect: async () => { await fileOperations.deletePaths([path]); onChanged?.() } },
    { kind: 'action', label: 'Delete permanently', shortcut: 'Shift+Del', onSelect: () => fileOperations.requestPermanentDelete([path]) },
    { kind: 'separator' },
    { kind: 'action', label: appState.isPinned(path) ? 'Unpin from sidebar' : 'Pin to sidebar', onSelect: () => appState.togglePin(path) },
    { kind: 'action', label: 'Properties', shortcut: 'Alt+Enter', onSelect: () => fileOperations.showProperties(path) },
    ...(appState.tags.length > 0 ? [{ kind: 'heading', label: 'Tags' } as ContextMenuItem] : []),
    ...appState.tags.map(
      (tag): ContextMenuItem => ({
        kind: 'toggle',
        label: tag.label,
        checked: appState.tagsFor(path).some((assigned) => assigned.id === tag.id),
        onSelect: () => appState.toggleTag(path, tag.id),
      }),
    ),
  ]
}

function parentOf(path: string): string {
  return path.slice(0, path.lastIndexOf('/')) || '/'
}
