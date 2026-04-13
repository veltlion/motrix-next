/**
 * @fileoverview Post-download auto-archive — pure classification logic.
 *
 * When a download completes and the file was NOT pre-classified (e.g. the URL
 * had no detectable extension like `/download?id=123`), this module determines
 * whether the file should be moved to a category directory based on its real
 * filename (as resolved by aria2 via Content-Disposition or URL path).
 *
 * Design:
 *   - Pure function, no side effects (actual file move is handled by caller)
 *   - Skips BT tasks (multi-file torrents have their own directory structure)
 *   - Skips files already in the correct category directory
 *   - Only processes the first file (files[0]) for single-file HTTP downloads
 */
import type { Aria2Task, FileCategory } from '@shared/types'
import { extractExtension, resolveCategory } from './fileCategory'

/**
 * Determines whether a completed task's file should be archived (moved)
 * to a category directory.
 *
 * @param task       - Completed aria2 task with resolved file paths
 * @param enabled    - Whether file classification is enabled
 * @param categories - Classification rules with absolute directory paths
 * @param baseDir    - Default download directory — only files here are candidates for archiving.
 *                     Files in user-specified custom paths are left untouched.
 * @returns `{ source, targetDir }` if archiving is needed, `null` otherwise
 */
export function resolveArchiveAction(
  task: Aria2Task,
  enabled: boolean,
  categories: FileCategory[],
  baseDir: string,
): { source: string; targetDir: string } | null {
  if (!enabled) return null
  if (categories.length === 0) return null

  // Skip BT tasks — multi-file torrents manage their own directory structure
  if (task.bittorrent) return null

  // Get the primary file's resolved path
  const firstFile = task.files?.[0]
  const filePath = firstFile?.path
  if (!filePath) return null

  // Extract filename from the full path
  const fileName = filePath.split(/[/\\]/).pop()
  if (!fileName) return null

  // Only archive files that are in the default download directory.
  // Files in user-specified custom paths were intentionally placed there — leave untouched.
  const fileDir = filePath.substring(0, filePath.length - fileName.length).replace(/[\\/]+$/, '')
  const normalizedBase = baseDir.replace(/[\\/]+$/, '')
  if (fileDir !== normalizedBase) return null

  // Determine category from real filename extension
  const ext = extractExtension(fileName)
  const category = resolveCategory(ext, categories)
  if (!category) return null

  return { source: filePath, targetDir: category.directory }
}
