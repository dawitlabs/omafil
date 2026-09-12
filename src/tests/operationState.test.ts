import { describe, expect, test } from 'bun:test'
import { operationPercent, reconcileOperation, remainingCutPaths, type FileOperation } from '../src/lib/operationState'

const operation: FileOperation = {
  id: 'operation-1', kind: 'move', state: 'queued', completedItems: 0, totalItems: 2,
  completedBytes: 0, totalBytes: null, currentName: null,
}

describe('operation event ordering', () => {
  test('a late command response cannot duplicate or regress a completed job', () => {
    const completed = { ...operation, state: 'completed' as const, completedItems: 2 }
    const state = reconcileOperation([], completed)
    expect(reconcileOperation(state, operation)).toEqual([completed])
    expect(reconcileOperation(state, { ...operation, state: 'running' })).toEqual([completed])
  })

  test('progress preserves cancellation feedback', () => {
    const state = [{ ...operation, state: 'running' as const, cancellationRequested: true }]
    const [next] = reconcileOperation(state, { ...operation, state: 'running', completedBytes: 1024 })
    expect(next.cancellationRequested).toBe(true)
    expect(next.completedBytes).toBe(1024)
    expect(reconcileOperation([next], operation)).toEqual([next])
  })

  test('a partial move keeps skipped and unfinished items on the cut clipboard', () => {
    expect(remainingCutPaths(['a', 'b', 'c'], [
      { sourcePath: 'a', destinationPath: 'out/a', skipped: false },
      { sourcePath: 'b', destinationPath: 'out/b', skipped: true },
    ])).toEqual(['b', 'c'])
  })

  test('100 percent is reserved for published completion', () => {
    expect(operationPercent(operation)).toBeUndefined()
    expect(operationPercent({ ...operation, state: 'running', totalBytes: 100, completedBytes: 100 })).toBe(99)
    expect(operationPercent({ ...operation, state: 'completed' })).toBe(100)
  })
})
