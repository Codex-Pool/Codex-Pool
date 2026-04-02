/// <reference types="node" />

import assert from 'node:assert/strict'
import test from 'node:test'

import { formatCompactNumber } from './compact-number.ts'

test('formatCompactNumber 在十亿级时切换到 B 单位', () => {
  assert.equal(formatCompactNumber(3_636_000_000, { locale: 'en-US' }), '3.6B')
})

test('formatCompactNumber 在百万级和千级使用 M/K 单位', () => {
  assert.equal(formatCompactNumber(2_500_000, { locale: 'en-US' }), '2.5M')
  assert.equal(formatCompactNumber(12_300, { locale: 'en-US' }), '12.3K')
})

test('formatCompactNumber 在千以下保留整数格式', () => {
  assert.equal(formatCompactNumber(950, { locale: 'en-US' }), '950')
})
