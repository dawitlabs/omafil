import { describe, expect, test } from 'bun:test'
import { basenameOf, parentOf, planUndo, pushEntry, UNDO_STACK_LIMIT, type UndoEntry } from '../src/lib/undo'

const moved = (sourcePath: string, destinationPath: string, skipped = false) => ({ sourcePath, destinationPath, skipped })

describe('parentOf and basenameOf', () => {
  test('split a path without losing the root', () => {
    expect(parentOf('/home/dave/notes.txt')).toBe('/home/dave')
    expect(parentOf('/home')).toBe('/')
    expect(basenameOf('/home/dave/notes.txt')).toBe('notes.txt')
    expect(basenameOf('/home/dave/')).toBe('dave')
  })
})

describe('planUndo', () => {
  test('sends moved items back to the folder they came from', () => {
    const plan = planUndo({ kind: 'move', results: [moved('/a/one.txt', '/b/one.txt'), moved('/c/two.txt', '/b/two.txt')] })

    expect(plan).toEqual({
      action: 'move',
      items: [
        { path: '/b/one.txt', destination: '/a' },
        { path: '/b/two.txt', destination: '/c' },
      ],
      label: 'Undo move of 2 items',
    })
  })

  test('trashes what a copy produced', () => {
    const plan = planUndo({ kind: 'copy', results: [moved('/a/one.txt', '/b/one.txt')] })

    expect(plan).toEqual({ action: 'trash', paths: ['/b/one.txt'], label: 'Undo copy of one item' })
  })

  test('ignores items the operation skipped', () => {
    expect(planUndo({ kind: 'move', results: [moved('/a/one.txt', '/b/one.txt', true)] })).toBeNull()
    expect(planUndo({ kind: 'copy', results: [moved('/a/one.txt', '/b/one.txt', true)] })).toBeNull()
  })

  test('renames back to the original name, not the original path', () => {
    const plan = planUndo({ kind: 'rename', renames: [{ from: '/a/old.txt', to: '/a/new.txt' }] })

    expect(plan).toEqual({ action: 'rename', renames: [{ path: '/a/new.txt', to: 'old.txt' }], label: 'Undo rename of one item' })
  })

  test('drops renames that did not change anything', () => {
    expect(planUndo({ kind: 'rename', renames: [{ from: '/a/same.txt', to: '/a/same.txt' }] })).toBeNull()
  })

  test('restores trashed items by their trash id', () => {
    const plan = planUndo({ kind: 'trash', ids: ['x1', 'x2'], count: 2 })

    expect(plan).toEqual({ action: 'restore', ids: ['x1', 'x2'], label: 'Restore 2 items' })
  })

  test('has nothing to restore when the trash reported no ids', () => {
    expect(planUndo({ kind: 'trash', ids: [], count: 3 })).toBeNull()
  })

  test('undoes a new folder by trashing it', () => {
    const plan = planUndo({ kind: 'create', path: '/a/New folder' })

    expect(plan).toEqual({ action: 'trash', paths: ['/a/New folder'], label: 'Undo new folder “New folder”' })
  })
})

describe('pushEntry', () => {
  test('keeps the newest entries once the limit is reached', () => {
    let stack: UndoEntry[] = []
    for (let index = 0; index < UNDO_STACK_LIMIT + 10; index += 1) {
      stack = pushEntry(stack, { kind: 'create', path: `/a/${index}` })
    }

    expect(stack).toHaveLength(UNDO_STACK_LIMIT)
    expect(stack.at(0)).toEqual({ kind: 'create', path: '/a/10' })
    expect(stack.at(-1)).toEqual({ kind: 'create', path: `/a/${UNDO_STACK_LIMIT + 9}` })
  })

  test('does not mutate the stack it was given', () => {
    const original: UndoEntry[] = []
    pushEntry(original, { kind: 'create', path: '/a/x' })

    expect(original).toHaveLength(0)
  })
})
