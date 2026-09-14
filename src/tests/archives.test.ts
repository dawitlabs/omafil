import { describe, expect, test } from 'bun:test'
import { isExtractable } from '../src/lib/archives'

describe('isExtractable', () => {
  test('accepts the archive formats Arch users actually meet', () => {
    for (const name of ['bundle.zip', 'src.tar.gz', 'pkg.tar.zst', 'old.7z', 'disc.iso', 'legacy.rar']) {
      expect(isExtractable(name)).toBe(true)
    }
  })

  test('ignores case', () => {
    expect(isExtractable('BUNDLE.ZIP')).toBe(true)
    expect(isExtractable('Source.TAR.XZ')).toBe(true)
  })

  test('rejects ordinary files and names without an extension', () => {
    for (const name of ['notes.txt', 'photo.png', 'Makefile', 'archive']) {
      expect(isExtractable(name)).toBe(false)
    }
  })

  test('does not treat a dotfile as its own extension', () => {
    expect(isExtractable('.zshrc')).toBe(false)
  })
})
