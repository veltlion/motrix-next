/**
 * @fileoverview Structural tests for tray menu focus-management behavior.
 *
 * Validates that ALL tray menu actions requiring UI interaction properly
 * include window show + focus calls in the Rust backend (tray.rs).
 * This prevents regression of the "New Task from tray doesn't bring
 * window to foreground" bug.
 *
 * HONESTY NOTE: These are structural source-code tests that inspect the
 * actual Rust implementation — not mocked stubs. They verify the REAL
 * code patterns that determine runtime behavior.
 */
import { describe, it, expect, beforeAll } from 'vitest'
import * as fs from 'node:fs'
import * as path from 'node:path'

const TAURI_ROOT = path.resolve(__dirname, '../../../../src-tauri')

describe('tray.rs — window focus management', () => {
  let traySource: string

  beforeAll(() => {
    const trayPath = path.join(TAURI_ROOT, 'src', 'tray.rs')
    traySource = fs.readFileSync(trayPath, 'utf-8')
  })

  /**
   * Actions that require the main window to be visible when triggered
   * from the tray. Each entry documents WHY it needs focus.
   */
  const UI_REQUIRED_ACTIONS = [
    {
      id: 'tray-new-task',
      reason: 'opens a modal dialog that is invisible without a visible parent window',
    },
  ] as const

  /**
   * Actions that operate on the engine without UI — these should NOT
   * force-show the window (user may intentionally keep it hidden).
   */
  const ENGINE_ONLY_ACTIONS = ['tray-resume-all', 'tray-pause-all'] as const

  // ---------- Core focus behavior ----------

  describe('UI-requiring actions must show + focus window before emit', () => {
    for (const action of UI_REQUIRED_ACTIONS) {
      describe(`"${action.id}"`, () => {
        it(`calls window.show() — ${action.reason}`, () => {
          // Extract the handler block for this action
          const handlerBlock = extractHandlerBlock(traySource, action.id)
          expect(handlerBlock).toBeTruthy()
          expect(handlerBlock).toContain('window.show()')
        })

        it('calls window.set_focus()', () => {
          const handlerBlock = extractHandlerBlock(traySource, action.id)
          expect(handlerBlock).toBeTruthy()
          expect(handlerBlock).toContain('window.set_focus()')
        })

        it('emits menu-event after focus (show/focus appear before emit)', () => {
          const handlerBlock = extractHandlerBlock(traySource, action.id)
          expect(handlerBlock).toBeTruthy()

          const showIdx = handlerBlock!.indexOf('window.show()')
          const focusIdx = handlerBlock!.indexOf('window.set_focus()')
          const emitIdx = handlerBlock!.indexOf('app.emit(')

          // All three must exist
          expect(showIdx).toBeGreaterThanOrEqual(0)
          expect(focusIdx).toBeGreaterThanOrEqual(0)
          expect(emitIdx).toBeGreaterThanOrEqual(0)

          // show and focus must come before emit
          expect(showIdx).toBeLessThan(emitIdx)
          expect(focusIdx).toBeLessThan(emitIdx)
        })
      })
    }
  })

  // ---------- macOS activation policy ----------

  it('"tray-new-task" sets macOS ActivationPolicy::Regular for dock visibility', () => {
    const handlerBlock = extractHandlerBlock(traySource, 'tray-new-task')
    expect(handlerBlock).toBeTruthy()
    expect(handlerBlock).toContain('ActivationPolicy::Regular')
  })

  // ---------- "show" handler consistency ----------

  it('"show" handler calls window.show() + set_focus()', () => {
    const handlerBlock = extractHandlerBlock(traySource, '"show"')
    expect(handlerBlock).toBeTruthy()
    expect(handlerBlock).toContain('window.show()')
    expect(handlerBlock).toContain('window.set_focus()')
  })

  // ---------- Engine-only actions should NOT force window ----------

  describe('engine-only actions should not force-show window', () => {
    for (const actionId of ENGINE_ONLY_ACTIONS) {
      it(`"${actionId}" does not call window.show()`, () => {
        const handlerBlock = extractHandlerBlock(traySource, actionId)
        expect(handlerBlock).toBeTruthy()
        expect(handlerBlock).not.toContain('window.show()')
      })
    }
  })

  // ---------- structural completeness ----------

  it('all 5 tray menu action IDs are handled in on_menu_event', () => {
    const expectedIds = ['show', 'tray-new-task', 'tray-resume-all', 'tray-pause-all', 'tray-quit']
    for (const id of expectedIds) {
      expect(traySource).toContain(`"${id}"`)
    }
  })
})

describe('Windows tray-menu-action handler parity (MainLayout.vue)', () => {
  let mainLayoutSource: string

  beforeAll(() => {
    const layoutPath = path.resolve(__dirname, '../../../../src/layouts/MainLayout.vue')
    mainLayoutSource = fs.readFileSync(layoutPath, 'utf-8')
  })

  it('menu-event "new-task" handler calls show() before showAddTaskDialog', () => {
    // Find the menu-event listener block
    const menuEventBlock = extractListenerBlock(mainLayoutSource, 'menu-event')
    expect(menuEventBlock).toBeTruthy()

    // Within the new-task case, show() must appear before showAddTaskDialog
    const newTaskCase = extractCaseBlock(menuEventBlock!, "'new-task'")
    expect(newTaskCase).toBeTruthy()
    expect(newTaskCase).toContain('.show()')
    expect(newTaskCase).toContain('showAddTaskDialog')

    const showIdx = newTaskCase!.indexOf('.show()')
    const dialogIdx = newTaskCase!.indexOf('showAddTaskDialog')
    expect(showIdx).toBeLessThan(dialogIdx)
  })

  it('menu-event "new-task" handler calls setFocus()', () => {
    const menuEventBlock = extractListenerBlock(mainLayoutSource, 'menu-event')
    expect(menuEventBlock).toBeTruthy()
    const newTaskCase = extractCaseBlock(menuEventBlock!, "'new-task'")
    expect(newTaskCase).toBeTruthy()
    expect(newTaskCase).toContain('setFocus')
  })

  it('tray-menu-action "new-task" handler (Windows) calls show + setFocus', () => {
    const trayActionBlock = extractListenerBlock(mainLayoutSource, 'tray-menu-action')
    expect(trayActionBlock).toBeTruthy()
    const newTaskCase = extractCaseBlock(trayActionBlock!, "'new-task'")
    expect(newTaskCase).toBeTruthy()
    expect(newTaskCase).toContain('.show()')
    expect(newTaskCase).toContain('setFocus')
  })
})

// ─── Helpers ────────────────────────────────────────────────────────

/**
 * Extract the handler block for a tray menu action from Rust source.
 * Matches patterns like: "tray-new-task" => { ... }
 */
function extractHandlerBlock(source: string, actionId: string): string | null {
  // Normalize: if actionId doesn't have quotes, add them
  const needle = actionId.includes('"') ? actionId : `"${actionId}"`
  // Search for the Rust match arm pattern: "id" => {
  // This avoids false matches on MenuItem::with_id(..., "id", ...) declarations.
  const armPattern = `${needle} =>`
  const idx = source.indexOf(armPattern)
  if (idx === -1) return null

  // Find the opening brace after =>
  const arrowIdx = idx + needle.length
  const braceStart = source.indexOf('{', arrowIdx)
  if (braceStart === -1) return null

  // Count balanced braces to find the end
  let depth = 0
  let end = braceStart
  for (let i = braceStart; i < source.length; i++) {
    if (source[i] === '{') depth++
    if (source[i] === '}') depth--
    if (depth === 0) {
      end = i
      break
    }
  }

  return source.slice(braceStart, end + 1)
}

/**
 * Extract the listener block for a Tauri event from Vue source.
 * Matches patterns like: listen<string>('event-name', async (event) => { ... })
 */
function extractListenerBlock(source: string, eventName: string): string | null {
  const needle = `'${eventName}'`
  const idx = source.indexOf(needle)
  if (idx === -1) return null

  // Find the arrow function body
  const arrowIdx = source.indexOf('=>', idx)
  if (arrowIdx === -1) return null
  const braceStart = source.indexOf('{', arrowIdx)
  if (braceStart === -1) return null

  // Count balanced braces
  let depth = 0
  let end = braceStart
  for (let i = braceStart; i < source.length; i++) {
    if (source[i] === '{') depth++
    if (source[i] === '}') depth--
    if (depth === 0) {
      end = i
      break
    }
  }

  return source.slice(braceStart, end + 1)
}

/**
 * Extract a switch-case block from source.
 * Returns everything from `case 'xxx':` to the next `break`/`case`/`}`.
 */
function extractCaseBlock(source: string, caseValue: string): string | null {
  const needle = `case ${caseValue}:`
  const idx = source.indexOf(needle)
  if (idx === -1) return null

  // Find the next break or next case
  const afterCase = source.slice(idx)
  const breakIdx = afterCase.indexOf('break')
  const nextCaseIdx = afterCase.indexOf('\n      case ', needle.length)

  let end: number
  if (breakIdx !== -1 && (nextCaseIdx === -1 || breakIdx < nextCaseIdx)) {
    end = breakIdx + 'break'.length
  } else if (nextCaseIdx !== -1) {
    end = nextCaseIdx
  } else {
    end = afterCase.length
  }

  return afterCase.slice(0, end)
}
