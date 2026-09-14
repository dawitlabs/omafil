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

describe('reconcileOperation gaps', () => {
  test('an event for an unknown job registers it instead of being dropped', () => {
    expect(reconcileOperation([], { ...operation, state: 'running' })).toHaveLength(1)
  })

  test('a queued event cannot pull a running job backwards', () => {
    const running = [{ ...operation, state: 'running' as const }]

    expect(reconcileOperation(running, operation)).toBe(running)
  })

  test('jobs that are not the one updating are left alone', () => {
    const other = { ...operation, id: 'operation-2' }
    const next = reconcileOperation([operation, other], { ...operation, state: 'running' })

    expect(next.find((job) => job.id === 'operation-2')).toBe(other)
  })
})

describe('operationPercent gaps', () => {
  test('is unknown while the total size is still being measured', () => {
    expect(operationPercent({ ...operation, totalBytes: null })).toBeUndefined()
  })

  test('falls back to counting items when the job carries no bytes', () => {
    expect(operationPercent({ ...operation, totalBytes: 0, totalItems: 4, completedItems: 1 })).toBe(25)
  })

  test('is zero, not NaN, for a job with neither bytes nor items', () => {
    expect(operationPercent({ ...operation, totalBytes: 0, totalItems: 0 })).toBe(0)
  })

  test('never reports a negative or over-full bar', () => {
    expect(operationPercent({ ...operation, totalBytes: 100, completedBytes: -5 })).toBe(0)
    expect(operationPercent({ ...operation, totalBytes: 100, completedBytes: 500 })).toBe(99)
  })
})

describe('remainingCutPaths gaps', () => {
  test('keeps every path when the move reported nothing', () => {
    expect(remainingCutPaths(['/a', '/b'], [])).toEqual(['/a', '/b'])
  })

  test('clears the clipboard once everything moved', () => {
    const results = [
      { sourcePath: '/a', destinationPath: '/x/a', skipped: false },
      { sourcePath: '/b', destinationPath: '/x/b', skipped: false },
    ]

    expect(remainingCutPaths(['/a', '/b'], results)).toEqual([])
  })
})
