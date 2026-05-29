import { describe, it, expect } from 'vitest'

describe('formatTokens helper', () => {
  // Helper function for testing (same logic as in TokenChart)
  function formatTokens(n: number): string {
    if (n < 1000) return String(n)
    if (n < 1_000_000) return `${(n / 1000).toFixed(1)}k`
    return `${(n / 1_000_000).toFixed(2)}M`
  }

  it('formats numbers less than 1000', () => {
    expect(formatTokens(0)).toBe('0')
    expect(formatTokens(999)).toBe('999')
  })

  it('formats thousands with k suffix', () => {
    expect(formatTokens(1000)).toBe('1.0k')
    expect(formatTokens(15000)).toBe('15.0k')
  })

  it('formats millions with M suffix', () => {
    expect(formatTokens(1000000)).toBe('1.00M')
    expect(formatTokens(2500000)).toBe('2.50M')
  })
})

describe('statusBounds helper', () => {
  // Pure function logic from LogsPage
  function statusBounds(filter: 'all' | 'ok' | 'error'): {
    statusMin?: number
    statusMax?: number
  } {
    if (filter === 'ok') return { statusMin: 200, statusMax: 399 }
    if (filter === 'error') return { statusMin: 400, statusMax: 599 }
    return {}
  }

  it('returns empty for all filter', () => {
    expect(statusBounds('all')).toEqual({})
  })

  it('returns 200-399 for ok filter', () => {
    expect(statusBounds('ok')).toEqual({ statusMin: 200, statusMax: 399 })
  })

  it('returns 400-599 for error filter', () => {
    expect(statusBounds('error')).toEqual({ statusMin: 400, statusMax: 599 })
  })
})

describe('csvEscape helper', () => {
  // CSV escaping from LogsPage
  function csvEscape(value: unknown): string {
    if (value == null) return ''
    const s = String(value)
    if (s.includes(',') || s.includes('"') || s.includes('\n')) {
      return `"${s.replace(/"/g, '""')}"`
    }
    return s
  }

  it('handles null/undefined', () => {
    expect(csvEscape(null)).toBe('')
    expect(csvEscape(undefined)).toBe('')
  })

  it('escapes strings with commas', () => {
    expect(csvEscape('hello,world')).toBe('"hello,world"')
  })

  it('escapes strings with quotes', () => {
    expect(csvEscape('say "hello"')).toBe('"say ""hello"""')
  })

  it('leaves plain strings unchanged', () => {
    expect(csvEscape('hello world')).toBe('hello world')
  })
})
