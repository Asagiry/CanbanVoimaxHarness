import { Language } from '../../../stores/languageStore';

export interface ParsedDiffLine {
  type: 'hunk' | 'add' | 'del' | 'context';
  text: string;
  oldNum: number | null;
  newNum: number | null;
}

/**
 * Parses unified diff into clean hunks, stripping all git metadata noise
 * (diff --git, index, --- /dev/null, +++ b/..., new file mode, etc.)
 */
export function parseCleanDiff(patch?: string | null): ParsedDiffLine[] {
  if (!patch || typeof patch !== 'string') return [];
  const lines = patch.split('\n');
  const result: ParsedDiffLine[] = [];
  let oldLine = 0;
  let newLine = 0;
  let seenFirstHunk = false;

  for (const line of lines) {
    // 1. Hunk header: starts with @@
    if (line.startsWith('@@')) {
      seenFirstHunk = true;
      const match = line.match(/@@\s+-(\d+)(?:,\d+)?\s+\+(\d+)(?:,\d+)?\s+@@/);
      if (match) {
        oldLine = parseInt(match[1], 10);
        newLine = parseInt(match[2], 10);
      }
      result.push({ type: 'hunk', text: line, oldNum: null, newNum: null });
      continue;
    }

    // 2. Ignore any git metadata noise before or between hunks
    if (!seenFirstHunk) {
      continue;
    }

    if (
      line.startsWith('diff --git') ||
      line.startsWith('index ') ||
      line.startsWith('--- ') ||
      line.startsWith('+++ ') ||
      line.startsWith('new file ') ||
      line.startsWith('deleted file ') ||
      line.startsWith('similarity index') ||
      line.startsWith('rename from') ||
      line.startsWith('rename to') ||
      line.startsWith('\\ No newline')
    ) {
      continue;
    }

    // 3. Diff content lines
    if (line.startsWith('+')) {
      result.push({ type: 'add', text: line.slice(1), oldNum: null, newNum: newLine });
      newLine++;
    } else if (line.startsWith('-')) {
      result.push({ type: 'del', text: line.slice(1), oldNum: oldLine, newNum: null });
      oldLine++;
    } else {
      const text = line.startsWith(' ') ? line.slice(1) : line;
      result.push({ type: 'context', text, oldNum: oldLine, newNum: newLine });
      oldLine++;
      newLine++;
    }
  }

  return result;
}

/**
 * Determines whether a file path belongs to internal CVH runtime artifacts,
 * hidden caches, or sensitive keys, while allowing standard user configurations.
 */
export const isCvhPath = (p: string): boolean => {
  if (!p) return false;
  const norm = p.replace(/\\/g, '/').trim();
  if (!norm) return false;

  // 1. Always hide runtime, secret, database, and log artifacts
  if (
    norm.includes('harness.db') ||
    norm.startsWith('worktrees/') ||
    norm.includes('/worktrees/') ||
    norm.endsWith('llm.json') ||
    norm.includes('/llm.json') ||
    norm.startsWith('.cvh/spill/') ||
    norm.includes('/.cvh/spill/') ||
    norm.startsWith('spill/') ||
    norm.includes('/spill/') ||
    norm.startsWith('.cvh/logs/') ||
    norm.includes('/.cvh/logs/') ||
    norm.startsWith('logs/') ||
    norm.includes('/logs/') ||
    norm.endsWith('.log') ||
    norm.startsWith('.cvh/dialogs/') ||
    norm.includes('/.cvh/dialogs/') ||
    norm.includes('/dialogs/') ||
    norm.endsWith('tasks.json') ||
    norm.includes('/tasks.json') ||
    norm.startsWith('.cvh/tasks/') ||
    norm.includes('/.cvh/tasks/') ||
    norm.includes('/tasks/') ||
    norm.endsWith('.secret') ||
    norm.includes('.secret/') ||
    norm.includes('/.secret') ||
    norm.endsWith('.key') ||
    norm.includes('.key/') ||
    norm.includes('/.key')
  ) {
    return true;
  }

  // 2. Bare .cvh directory itself is hidden
  if (norm === '.cvh' || norm === '.cvh/' || norm.endsWith('/.cvh') || norm.endsWith('/.cvh/')) {
    return true;
  }

  // 3. Explicitly allowed CVH configuration files & directories:
  //    .cvh/rules/, .cvh/agents/, .cvh/settings/, .cvh/tools/, .cvh/hooks/,
  //    .cvh/boards.json, .cvh/pipeline.json
  const isCvhConfig =
    norm.startsWith('.cvh/rules/') ||
    norm.includes('/.cvh/rules/') ||
    norm.startsWith('.cvh/agents/') ||
    norm.includes('/.cvh/agents/') ||
    norm.startsWith('.cvh/settings/') ||
    norm.includes('/.cvh/settings/') ||
    norm.startsWith('.cvh/tools/') ||
    norm.includes('/.cvh/tools/') ||
    norm.startsWith('.cvh/hooks/') ||
    norm.includes('/.cvh/hooks/') ||
    norm === '.cvh/boards.json' ||
    norm.endsWith('/.cvh/boards.json') ||
    norm === '.cvh/pipeline.json' ||
    norm.endsWith('/.cvh/pipeline.json');

  if (isCvhConfig) {
    return false;
  }

  // 4. Any other file inside .cvh is considered internal runtime and hidden
  if (norm.startsWith('.cvh/') || norm.includes('/.cvh/')) {
    return true;
  }

  // 5. Standard workspace files outside .cvh are visible
  return false;
};

export function formatFilesCount(count: number, lang: Language = 'en'): string {
  if (lang === 'en') {
    return count === 1 ? '1 file' : `${count} files`;
  }
  const mod10 = count % 10;
  const mod100 = count % 100;
  if (mod100 >= 11 && mod100 <= 19) return `${count} файлов`;
  if (mod10 === 1) return `${count} файл`;
  if (mod10 >= 2 && mod10 <= 4) return `${count} файла`;
  return `${count} файлов`;
}

export const IMAGE_EXTENSIONS = new Set(['png', 'jpg', 'jpeg', 'webp', 'gif', 'svg', 'bmp', 'ico', 'avif']);

export function isImageFile(filePath: string): boolean {
  const ext = filePath.split('.').pop()?.toLowerCase();
  return ext ? IMAGE_EXTENSIONS.has(ext) : false;
}

export function formatBytes(bytes?: number | null): string {
  if (bytes === null || bytes === undefined) return '';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}
