/// <reference types="node" />

import assert from 'node:assert/strict'
import test from 'node:test'
import { readFile } from 'node:fs/promises'

const MODEL_ROUTING_PATH = new URL('./ModelRouting.tsx', import.meta.url)

test('ModelRouting stays on the antigravity page and dialog archetypes', async () => {
  const source = await readFile(MODEL_ROUTING_PATH, 'utf8')

  assert.doesNotMatch(
    source,
    /POOL_SECTION_CLASS_NAME/,
    'ModelRouting should not depend on old pool section containers',
  )
  assert.match(
    source,
    /AntigravityDialogShell/,
    'ModelRouting dialogs should use the shared antigravity dialog shell',
  )
  assert.match(source, /DockedPageIntro/, 'ModelRouting should use DockedPageIntro as the page entry')
  assert.match(source, /PagePanel/, 'ModelRouting should keep PagePanel surfaces')
  assert.match(source, /SectionHeader/, 'ModelRouting should keep SectionHeader structure')
  assert.match(
    source,
    /modelRoutingPage\.claudeCode\.title/,
    'ModelRouting should include the dedicated Claude Code panel copy',
  )
  assert.match(
    source,
    /modelRoutingApi\.getClaudeCodeSettings\(\)/,
    'ModelRouting should load Claude Code routing settings from the admin API',
  )
  assert.match(
    source,
    /modelRoutingApi\.updateClaudeCodeSettings\(/,
    'ModelRouting should save Claude Code routing settings through the admin API',
  )
  assert.match(source, /modelRoutingPage\.claudeCode\.families\.opus/, 'ModelRouting should render the Opus mapping slot')
  assert.match(source, /modelRoutingPage\.claudeCode\.families\.sonnet/, 'ModelRouting should render the Sonnet mapping slot')
  assert.match(source, /modelRoutingPage\.claudeCode\.families\.haiku/, 'ModelRouting should render the Haiku mapping slot')
  assert.doesNotMatch(source, /components\/ui\/card/, 'ModelRouting should not depend on legacy Card wrapper surfaces')
  assert.doesNotMatch(source, /bg-content2\/16/, 'ModelRouting dialog panels should not use ad-hoc surface overrides')
})

test('ModelSelector should stay on shared surface primitives', async () => {
  const selectorSource = await readFile(new URL('../components/model-routing/model-selector.tsx', import.meta.url), 'utf8')

  assert.match(selectorSource, /SurfaceInset/, 'ModelSelector empty states should use shared SurfaceInset')
  assert.match(selectorSource, /SurfaceCard/, 'ModelSelector selected items should use shared SurfaceCard')
  assert.doesNotMatch(selectorSource, /rounded-lg border/, 'ModelSelector should not keep self-drawn bordered list items')
})
