export type BulkRenameRule = { find: string; replace: string; pattern: string; start: number }
export type BulkRenamePlan = { from: string; to: string; issue: string | null }

function splitName(name: string): { base: string; ext: string } {
  const dot = name.lastIndexOf('.')

  if (dot <= 0) return { base: name, ext: '' }

  return { base: name.slice(0, dot), ext: name.slice(dot) }
}

function issueFor(to: string, duplicates: Map<string, number>, untouched: Set<string>): string | null {
  if (to === '' || to === '.' || to === '..') return 'Name is empty'
  if (to.includes('/')) return 'Names cannot contain /'
  if ((duplicates.get(to) ?? 0) > 1) return 'Duplicate name'
  if (untouched.has(to)) return 'Already exists'

  return null
}

/** `taken` is every name in the folder; names being renamed are allowed to swap. */
export function planBulkRename(names: string[], rule: BulkRenameRule, taken: string[] = []): BulkRenamePlan[] {
  const width = String(rule.start + names.length - 1).length
  const targets = names.map((name, index) => {
    const { base, ext } = splitName(name)
    const replaced = rule.find ? base.replaceAll(rule.find, rule.replace) : base
    const counter = String(rule.start + index).padStart(width, '0')

    return rule.pattern.replaceAll('{name}', replaced).replaceAll('{ext}', ext).replaceAll('{n}', counter).trim()
  })
  const duplicates = new Map<string, number>()
  for (const target of targets) duplicates.set(target, (duplicates.get(target) ?? 0) + 1)
  const untouched = new Set(taken.filter((name) => !names.includes(name)))

  return names.map((from, index) => ({ from, to: targets[index], issue: issueFor(targets[index], duplicates, untouched) }))
}

export function isApplicable(plan: BulkRenamePlan[]): boolean {
  return plan.some((item) => item.from !== item.to) && plan.every((item) => item.issue === null)
}
