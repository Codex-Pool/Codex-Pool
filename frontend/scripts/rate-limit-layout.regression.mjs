import assert from 'node:assert/strict'

import {
  buildCompactRateLimitRows,
  formatCompactRateLimitReset,
  getCompactRateLimitBarColor,
} from '../src/features/accounts/rate-limit-layout.ts'

const rows = buildCompactRateLimitRows(
  [
    { bucket: 'github', remainingPercent: 55.2, resetsAt: '2026-03-11T10:20:00Z' },
    { bucket: 'one_week', remainingPercent: 48.2, resetsAt: '2026-03-18T10:20:00Z' },
    { bucket: 'five_hours', remainingPercent: 81.5, resetsAt: '2026-03-11T15:40:00Z' },
  ],
  {
    locale: 'en-US',
    fiveHoursLabel: '5h',
    oneWeekLabel: '7d',
    noResetText: '--',
    timeZone: 'UTC',
  },
)

assert.deepEqual(
  rows.map((row) => ({ bucket: row.bucket, span: row.span, remainingText: row.remainingText, bucketText: row.bucketText })),
  [
    { bucket: 'five_hours', span: 'half', remainingText: '81.5%', bucketText: '5h' },
    { bucket: 'one_week', span: 'half', remainingText: '48.2%', bucketText: '7d' },
  ],
)

assert.equal(
  formatCompactRateLimitReset('five_hours', '2026-03-11T15:40:00Z', {
    locale: 'en-US',
    noResetText: '--',
    timeZone: 'UTC',
  }),
  '15:40',
)

assert.equal(
  formatCompactRateLimitReset('one_week', '2026-03-18T10:20:00Z', {
    locale: 'en-US',
    noResetText: '--',
    timeZone: 'UTC',
  }),
  '2026-03-18 10:20',
)

assert.deepEqual(
  buildCompactRateLimitRows(
    [{ bucket: 'one_week', remainingPercent: 12.3, resetsAt: '2026-03-18T10:20:00Z' }],
    {
      locale: 'en-US',
      fiveHoursLabel: '5h',
      oneWeekLabel: '7d',
      noResetText: '--',
      timeZone: 'UTC',
    },
  ).map((row) => row.span),
  ['full'],
)

assert.equal(getCompactRateLimitBarColor(100), 'hsl(120 72% 44%)')
assert.equal(getCompactRateLimitBarColor(50), 'hsl(60 72% 44%)')
assert.equal(getCompactRateLimitBarColor(0), 'hsl(0 72% 44%)')

console.log('rate limit layout regression checks passed')
