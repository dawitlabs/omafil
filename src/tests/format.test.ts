import { describe, expect, test } from 'bun:test'
import { formatBytes, formatCount, formatItems, formatModified, parentFolder, typeLabel } from '../src/lib/format'

describe('formatBytes', () => {
  test('keeps bytes whole and climbs one unit at a time', () => {
    expect(formatBytes(0)).toBe('0 B')
    expect(formatBytes(999)).toBe('999 B')
    expect(formatBytes(1024)).toBe('1.0 KB')
    expect(formatBytes(1024 * 1024)).toBe('1.0 MB')
    expect(formatBytes(1024 ** 4)).toBe('1.0 TB')
  })

  test('drops the decimal once the number is big enough to not need it', () => {
    expect(formatBytes(1024 * 9.5)).toBe('9.5 KB')
    expect(formatBytes(1024 * 10)).toBe('10 KB')
  })

  test('stops at the largest unit it knows instead of inventing one', () => {
    expect(formatBytes(1024 ** 5)).toBe('1024 TB')
  })
})

describe('formatItems', () => {
  test('agrees with the count it is given', () => {
    expect(formatItems(0)).toBe('0 items')
    expect(formatItems(1)).toBe('1 item')
    expect(formatItems(2)).toBe('2 items')
  })

  test('groups thousands so long folders stay readable', () => {
    expect(formatCount(12345)).toBe((12345).toLocaleString())
  })
})

describe('formatModified', () => {
  test('renders nothing for an unknown time rather than the epoch', () => {
    expect(formatModified(null)).toBe('')
  })

  test('renders a real timestamp', () => {
    expect(formatModified(1_700_000_000)).not.toBe('')
  })
})

describe('parentFolder', () => {
  test('walks up one level and never past the root', () => {
    expect(parentFolder('/home/dave/notes.txt')).toBe('/home/dave')
    expect(parentFolder('/home')).toBe('/')
  })
})

describe('typeLabel', () => {
  test('names folders as folders whatever they are called', () => {
    expect(typeLabel('archive.zip', true)).toBe('File folder')
  })

  test('uses the extension when there is one', () => {
    expect(typeLabel('notes.txt', false)).toBe('TXT file')
    expect(typeLabel('bundle.tar.gz', false)).toBe('GZ file')
  })

  test('falls back for names with no extension and for dotfiles', () => {
    expect(typeLabel('Makefile', false)).toBe('File')
    expect(typeLabel('.zshrc', false)).toBe('File')
  })
})
