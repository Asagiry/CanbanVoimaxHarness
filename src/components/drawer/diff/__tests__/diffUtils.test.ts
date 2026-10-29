import { describe, it, expect } from 'vitest';
import {
  parseCleanDiff,
  isCvhPath,
  formatBytes,
  formatFilesCount,
  isImageFile,
} from '../utils';

describe('parseCleanDiff (Unified Git Diff Parsing)', () => {
  it('returns empty array when diff patch is empty or null', () => {
    expect(parseCleanDiff('')).toEqual([]);
    expect(parseCleanDiff(null)).toEqual([]);
    expect(parseCleanDiff(undefined)).toEqual([]);
  });

  it('strips git metadata headers before the first hunk', () => {
    const diff = [
      'diff --git a/src/app.ts b/src/app.ts',
      'index 1234567..89abcdef 100644',
      '--- a/src/app.ts',
      '+++ b/src/app.ts',
      '@@ -1,3 +1,4 @@',
      ' import React from \'react\';',
      '+import { useState } from \'react\';',
      '-export const old = 1;',
      '+export const current = 2;',
      ' export default App;',
    ].join('\n');

    const lines = parseCleanDiff(diff);
    expect(lines).toHaveLength(6);
    expect(lines[0]).toEqual({
      type: 'hunk',
      text: '@@ -1,3 +1,4 @@',
      oldNum: null,
      newNum: null,
    });
  });

  it('calculates line numbers correctly across add, del, and context lines', () => {
    const diff = [
      '@@ -10,3 +20,4 @@',
      ' line A',
      '-line B old',
      '+line B new',
      '+line B extra',
      ' line C',
    ].join('\n');

    const lines = parseCleanDiff(diff);
    expect(lines).toHaveLength(6);

    // Hunk
    expect(lines[0]).toEqual({ type: 'hunk', text: '@@ -10,3 +20,4 @@', oldNum: null, newNum: null });
    // Context line A (old: 10, new: 20)
    expect(lines[1]).toEqual({ type: 'context', text: 'line A', oldNum: 10, newNum: 20 });
    // Deleted line B old (old: 11, new: null)
    expect(lines[2]).toEqual({ type: 'del', text: 'line B old', oldNum: 11, newNum: null });
    // Added line B new (old: null, new: 21)
    expect(lines[3]).toEqual({ type: 'add', text: 'line B new', oldNum: null, newNum: 21 });
    // Added line B extra (old: null, new: 22)
    expect(lines[4]).toEqual({ type: 'add', text: 'line B extra', oldNum: null, newNum: 22 });
    // Context line C (old: 12, new: 23)
    expect(lines[5]).toEqual({ type: 'context', text: 'line C', oldNum: 12, newNum: 23 });
  });

  it('ignores inter-hunk git noise such as \\ No newline at end of file', () => {
    const diff = [
      '@@ -1,2 +1,2 @@',
      '-old line',
      '\\ No newline at end of file',
      '+new line',
      '\\ No newline at end of file',
    ].join('\n');

    const lines = parseCleanDiff(diff);
    expect(lines).toHaveLength(3);
    expect(lines[1]).toEqual({ type: 'del', text: 'old line', oldNum: 1, newNum: null });
    expect(lines[2]).toEqual({ type: 'add', text: 'new line', oldNum: null, newNum: 1 });
  });

  it('handles multiple hunks sequentially', () => {
    const diff = [
      '@@ -1,2 +1,2 @@',
      '-first',
      '+FIRST',
      ' context',
      '@@ -10,2 +10,2 @@',
      '-second',
      '+SECOND',
      ' end',
    ].join('\n');

    const lines = parseCleanDiff(diff);
    expect(lines).toHaveLength(8);
    expect(lines[0].type).toBe('hunk');
    expect(lines[4].type).toBe('hunk');
    expect(lines[5]).toEqual({ type: 'del', text: 'second', oldNum: 10, newNum: null });
    expect(lines[6]).toEqual({ type: 'add', text: 'SECOND', oldNum: null, newNum: 10 });
  });
});

describe('isCvhPath (Path Filtering & Exclusion Invariant)', () => {
  it('returns false for empty or whitespace paths', () => {
    expect(isCvhPath('')).toBe(false);
    expect(isCvhPath('   ')).toBe(false);
  });

  it('hides internal runtime, databases, secrets, and transient worktrees', () => {
    expect(isCvhPath('.cvh/harness.db')).toBe(true);
    expect(isCvhPath('C:\\repo\\.cvh\\harness.db')).toBe(true);
    expect(isCvhPath('worktrees/feat-branch/src/index.ts')).toBe(true);
    expect(isCvhPath('.cvh/spill/2026-03-01.log')).toBe(true);
    expect(isCvhPath('.cvh/logs/trace.log')).toBe(true);
    expect(isCvhPath('.cvh/llm.json')).toBe(true);
    expect(isCvhPath('llm.json')).toBe(true);
    expect(isCvhPath('api.secret')).toBe(true);
    expect(isCvhPath('private.key')).toBe(true);
    expect(isCvhPath('.cvh/tasks/task-001.json')).toBe(true);
    expect(isCvhPath('.cvh/dialogs/task-001.json')).toBe(true);
    expect(isCvhPath('.cvh')).toBe(true);
    expect(isCvhPath('.cvh/')).toBe(true);
  });

  it('allows explicit user and agent configuration files inside .cvh/', () => {
    expect(isCvhPath('.cvh/rules/general.md')).toBe(false);
    expect(isCvhPath('.cvh/agents/developer/persona.md')).toBe(false);
    expect(isCvhPath('.cvh/settings/tool-policy.json')).toBe(false);
    expect(isCvhPath('.cvh/tools/custom-tool.json')).toBe(false);
    expect(isCvhPath('.cvh/hooks/on-enter.ps1')).toBe(false);
    expect(isCvhPath('.cvh/boards.json')).toBe(false);
    expect(isCvhPath('.cvh/pipeline.json')).toBe(false);
  });

  it('hides unwhitelisted arbitrary internal files in .cvh/', () => {
    expect(isCvhPath('.cvh/cache/index.dat')).toBe(true);
    expect(isCvhPath('.cvh/unknown-folder/file.txt')).toBe(true);
  });

  it('allows standard user source code and project files outside .cvh', () => {
    expect(isCvhPath('src/App.tsx')).toBe(false);
    expect(isCvhPath('src-tauri/src/main.rs')).toBe(false);
    expect(isCvhPath('package.json')).toBe(false);
    expect(isCvhPath('README.md')).toBe(false);
  });
});

describe('formatBytes (Byte Formatting Utility)', () => {
  it('returns empty string for null or undefined input', () => {
    expect(formatBytes(null)).toBe('');
    expect(formatBytes(undefined)).toBe('');
  });

  it('formats bytes under 1024 as B', () => {
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(128)).toBe('128 B');
    expect(formatBytes(1023)).toBe('1023 B');
  });

  it('formats values in KB with 1 decimal place', () => {
    expect(formatBytes(1024)).toBe('1.0 KB');
    expect(formatBytes(1536)).toBe('1.5 KB');
    expect(formatBytes(2048)).toBe('2.0 KB');
    expect(formatBytes(1024 * 50)).toBe('50.0 KB');
  });

  it('formats values in MB with 2 decimal places', () => {
    expect(formatBytes(1024 * 1024)).toBe('1.00 MB');
    expect(formatBytes(1024 * 1024 * 2.5)).toBe('2.50 MB');
    expect(formatBytes(10485760)).toBe('10.00 MB');
  });
});

describe('formatFilesCount & isImageFile Helpers', () => {
  it('formats file counts with English pluralization', () => {
    expect(formatFilesCount(1, 'en')).toBe('1 file');
    expect(formatFilesCount(5, 'en')).toBe('5 files');
    expect(formatFilesCount(0, 'en')).toBe('0 files');
  });

  it('formats file counts with Russian pluralization rules', () => {
    expect(formatFilesCount(1, 'ru')).toBe('1 файл');
    expect(formatFilesCount(2, 'ru')).toBe('2 файла');
    expect(formatFilesCount(4, 'ru')).toBe('4 файла');
    expect(formatFilesCount(5, 'ru')).toBe('5 файлов');
    expect(formatFilesCount(11, 'ru')).toBe('11 файлов');
    expect(formatFilesCount(21, 'ru')).toBe('21 файл');
  });

  it('identifies image files by extension', () => {
    expect(isImageFile('photo.png')).toBe(true);
    expect(isImageFile('icon.SVG')).toBe(true);
    expect(isImageFile('banner.webp')).toBe(true);
    expect(isImageFile('document.pdf')).toBe(false);
    expect(isImageFile('code.ts')).toBe(false);
  });
});
