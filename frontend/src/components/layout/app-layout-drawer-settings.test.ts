/// <reference types="node" />

import assert from 'node:assert/strict'
import test from 'node:test'
import { readFile } from 'node:fs/promises'

const APP_LAYOUT_PATH = new URL('./AppLayout.tsx', import.meta.url)
const APP_PATH = new URL('../../App.tsx', import.meta.url)

test('app chrome exposes drawer placement preferences from the UI preferences layer', async () => {
  const [layoutSource, appSource] = await Promise.all([
    readFile(APP_LAYOUT_PATH, 'utf8'),
    readFile(APP_PATH, 'utf8'),
  ])

  assert.match(
    layoutSource,
    /useUiPreferences|setDrawerPlacement/,
    'AppLayout should read and update drawer placement preferences',
  )
  assert.match(
    layoutSource,
    /bottom|right|left|top/,
    'AppLayout should expose the supported drawer placement options',
  )
  assert.match(
    appSource,
    /UiPreferencesProvider/,
    'App should mount the UI preferences provider near the app shell',
  )
})

test('app chrome keeps the expanded desktop sidebar on a standard Tailwind width token instead of a custom pixel width', async () => {
  const layoutSource = await readFile(APP_LAYOUT_PATH, 'utf8')

  assert.match(
    layoutSource,
    /collapsed \? "w-\[68px\]" : "w-56"/,
    'AppLayout should keep the collapsed width as-is and narrow the expanded desktop sidebar to the next smaller standard Tailwind width token',
  )
  assert.match(
    layoutSource,
    /md:w-56/,
    'AppLayout should use the same smaller standard width token when the top or bottom drawer placement re-enters the desktop sidebar layout',
  )
  assert.doesNotMatch(
    layoutSource,
    /collapsed \? "w-\[68px\]" : "w-\[(?:\d+(?:\.\d+)?)px\]"/,
    'AppLayout should not switch the expanded sidebar width branch to a custom pixel literal when a standard spacing token is sufficient',
  )
  assert.doesNotMatch(
    layoutSource,
    /\? "md:w-\[(?:\d+(?:\.\d+)?)px\]" : ""/,
    'AppLayout should not switch the desktop re-entry width for top or bottom drawer placement to a custom pixel literal when a standard spacing token is sufficient',
  )
})
