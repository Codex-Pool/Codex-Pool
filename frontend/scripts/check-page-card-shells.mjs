import { readFile, readdir } from 'node:fs/promises'
import path from 'node:path'
import process from 'node:process'

const ROOT = path.resolve(import.meta.dirname, '..')
const TARGET_DIRECTORIES = ['src/pages', 'src/features', 'src/tenant']
const IGNORE_RELATIVE_PATHS = new Set(['src/pages/Login.tsx'])
const CARD_IMPORT_PATTERN = /import\s*\{[\s\S]*?\bCard\b[\s\S]*?\}\s*from\s*['"]@heroui\/react['"]/m

async function walk(directory) {
  const entries = await readdir(directory, { withFileTypes: true })
  const files = await Promise.all(entries.map(async (entry) => {
    const absolutePath = path.join(directory, entry.name)
    if (entry.isDirectory()) {
      return walk(absolutePath)
    }
    return absolutePath.endsWith('.tsx') ? [absolutePath] : []
  }))

  return files.flat()
}

const candidateFiles = (
  await Promise.all(TARGET_DIRECTORIES.map((directory) => walk(path.join(ROOT, directory))))
).flat()

const violations = []

for (const absolutePath of candidateFiles) {
  const relativePath = path.relative(ROOT, absolutePath).replaceAll(path.sep, '/')
  if (IGNORE_RELATIVE_PATHS.has(relativePath)) {
    continue
  }

  const source = await readFile(absolutePath, 'utf8')
  if (CARD_IMPORT_PATTERN.test(source)) {
    violations.push(relativePath)
  }
}

if (violations.length > 0) {
  console.error('Use the shared local card primitive instead of raw HeroUI Card imports in page-level files:')
  for (const file of violations) {
    console.error(`- ${file}`)
  }
  process.exit(1)
}

console.log('Page-level card shell guard passed.')
