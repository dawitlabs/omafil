import { describe, expect, test } from 'bun:test'
import { isApplicable, planBulkRename } from '../src/lib/bulkRename'

const rule = { find: '', replace: '', pattern: '{name}{ext}', start: 1 }

describe('bulk rename planning', () => {
  test('numbers, replaces and keeps extensions', () => {
    const plan = planBulkRename(['IMG_1.jpg', 'IMG_2.jpg', '.env', 'notes'], { find: 'IMG_', replace: 'shot ', pattern: 'photo-{n} {name}{ext}', start: 9 })

    expect(plan.map((item) => item.to)).toEqual(['photo-09 shot 1.jpg', 'photo-10 shot 2.jpg', 'photo-11 .env', 'photo-12 notes'])
    expect(isApplicable(plan)).toBe(true)
  })

  test('flags collisions and refuses a plan that changes nothing', () => {
    const duplicates = planBulkRename(['a.txt', 'b.txt'], { ...rule, pattern: 'same{ext}' })
    const clash = planBulkRename(['a.txt'], { ...rule, find: 'a', replace: 'b' }, ['a.txt', 'b.txt'])
    const swap = planBulkRename(['a.txt', 'b.txt'], { ...rule, pattern: '{n}{ext}' }, ['a.txt', 'b.txt'])

    expect(duplicates.map((item) => item.issue)).toEqual(['Duplicate name', 'Duplicate name'])
    expect(clash[0].issue).toBe('Already exists')
    expect(planBulkRename(['a.txt'], { ...rule, pattern: '' })[0].issue).toBe('Name is empty')
    expect(isApplicable(planBulkRename(['a.txt'], rule))).toBe(false)
    expect(isApplicable(swap)).toBe(true)
  })
})
