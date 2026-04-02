/// <reference types="node" />

import assert from 'node:assert/strict'
import test from 'node:test'

async function loadI18nModule() {
  return import('./i18n.ts')
}

test('normalizeLanguage falls back to English for unsupported locales and keeps supported mappings stable', async () => {
  const module = await loadI18nModule()

  assert.equal(
    typeof module.normalizeLanguage,
    'function',
    'i18n should expose the language normalization helper for regression coverage',
  )

  assert.equal(module.normalizeLanguage('fr-FR'), 'en')
  assert.equal(module.normalizeLanguage('en-US'), 'en')
  assert.equal(module.normalizeLanguage('zh-TW'), 'zh-CN')
  assert.equal(module.normalizeLanguage(undefined), 'en')
})

test('language detection keeps manual choice ahead of browser and document hints', async () => {
  const { default: i18n } = await loadI18nModule()
  const detection = i18n.options.detection as
    | {
      order?: string[]
      lookupLocalStorage?: string
    }
    | undefined

  assert.deepEqual(detection?.order, ['localStorage', 'navigator', 'htmlTag'])
  assert.equal(detection?.lookupLocalStorage, 'codex-ui-language')
})
