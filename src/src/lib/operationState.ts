export type TransferResult = { sourcePath: string; destinationPath: string; skipped: boolean }

export type FileOperation = {
  id: string
  kind: 'copy' | 'move' | 'compress' | 'extract'
  state: 'queued' | 'running' | 'completed' | 'cancelled' | 'failed'
  completedItems: number
  totalItems: number
  completedBytes: number
  totalBytes: number | null
  currentName: string | null
  results?: TransferResult[]
  error?: string | null
  cancellationRequested?: boolean
  cancellationError?: string | null
}

export function isTerminal(operation: FileOperation): boolean {
  return operation.state === 'completed' || operation.state === 'cancelled' || operation.state === 'failed'
}

/** Native events can arrive before the command response that registered the job. */
export function reconcileOperation(operations: FileOperation[], update: FileOperation): FileOperation[] {
  const previous = operations.find((operation) => operation.id === update.id)
  if (!previous) return [...operations, update]
  if (isTerminal(previous) || (previous.state === 'running' && update.state === 'queued')) return operations
  return operations.map((operation) => operation.id === update.id
    ? { ...update, cancellationRequested: previous.cancellationRequested, cancellationError: previous.cancellationError }
    : operation)
}

export function remainingCutPaths(paths: string[], results: TransferResult[]): string[] {
  const moved = new Set(results.filter((result) => !result.skipped).map((result) => result.sourcePath))
  return paths.filter((path) => !moved.has(path))
}

export function operationPercent(operation: FileOperation): number | undefined {
  if (operation.state === 'completed') return 100
  if (operation.totalBytes === null) return undefined
  const fraction = operation.totalBytes > 0
    ? operation.completedBytes / operation.totalBytes
    : operation.totalItems > 0 ? operation.completedItems / operation.totalItems : 0
  return Math.max(0, Math.min(99, fraction * 100))
}
