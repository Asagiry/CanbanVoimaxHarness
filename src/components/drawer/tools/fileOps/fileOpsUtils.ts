import { ToolExecution } from '../../../../types';
import { isToolFailed } from '../shared';

export interface FileOpsParsedInfo {
  toolName: string;
  params: Record<string, unknown>;
  isFileOp: boolean;
  isReadOp: boolean;
  isEditTool: boolean;
  isDiffTool: boolean;
  rawFilePath: string;
  normalizedPath: string;
  fileName: string;
  dirPath: string;
  oldString?: string;
  newString?: string;
  additions: number;
  deletions: number;
  opLabel: 'Created' | 'Edited';
  ext: string;
  lineRangeBadge: string | null;
  startLine?: number;
  isFailed: boolean;
}

export function parseFileOpsInfo(tool: ToolExecution): FileOpsParsedInfo {
  const toolName = tool.toolName ?? 'file_op';
  const params: Record<string, unknown> =
    tool.params && typeof tool.params === 'object'
      ? (tool.params as Record<string, unknown>)
      : {};

  const isFileOp =
    toolName === 'write_file' ||
    toolName === 'write' ||
    toolName === 'edit_file' ||
    toolName === 'edit' ||
    toolName === 'str_replace';

  const isReadOp =
    toolName === 'read_file' ||
    toolName === 'read' ||
    toolName === 'fetch_file';

  const isEditTool =
    toolName === 'edit_file' ||
    toolName === 'str_replace' ||
    toolName === 'edit';
  const isDiffTool = isEditTool;

  const rawFilePath =
    (params?.file_path as string) ||
    (params?.path as string) ||
    (params?.filePath as string) ||
    (params?.target_file as string) ||
    '';

  const normalizedPath = rawFilePath.replace(/\\/g, '/');
  const fileName = normalizedPath.split('/').pop() || rawFilePath;
  const dirPath = normalizedPath.substring(0, normalizedPath.length - fileName.length).replace(/\/$/, '');

  const oldString =
    (params?.old_string as string | undefined) ??
    (params?.old_str as string | undefined) ??
    (params?.before as string | undefined);

  const newString =
    (params?.new_string as string | undefined) ??
    (params?.new_str as string | undefined) ??
    (params?.content as string | undefined);

  let additions = 0;
  let deletions = 0;

  if (oldString !== undefined && oldString !== '') {
    deletions = oldString.split('\n').length;
  }
  if (newString !== undefined && newString !== '') {
    additions = newString.split('\n').length;
  }

  if (additions === 0 && deletions === 0) {
    if (typeof params?.lines === 'number') {
      additions = params.lines;
    } else if (tool.stdout) {
      const match = tool.stdout.match(/\+(\d+)\s*-(\d+)/);
      if (match) {
        additions = parseInt(match[1], 10);
        deletions = parseInt(match[2], 10);
      }
    }
  }

  const isFailed = isToolFailed(tool);

  let opLabel: 'Created' | 'Edited' = 'Edited';
  if (isEditTool) {
    opLabel = 'Edited';
  } else {
    const op = (params?.operation as string)?.toLowerCase();
    if (op === 'create' || params?.is_new === true) {
      opLabel = 'Created';
    } else if (op === 'update' || deletions > 0 || params?.before !== undefined) {
      opLabel = 'Edited';
    } else {
      const stdoutLower = (tool.stdout ?? '').toLowerCase();
      if (stdoutLower.includes('created') || stdoutLower.includes('new file')) {
        opLabel = 'Created';
      } else if (stdoutLower.includes('update') || stdoutLower.includes('overwrit') || stdoutLower.includes('replaced')) {
        opLabel = 'Edited';
      } else {
        opLabel = deletions === 0 ? 'Created' : 'Edited';
      }
    }
  }

  const ext = fileName.includes('.') ? fileName.split('.').pop()?.toLowerCase() || '' : '';

  const offsetVal =
    params?.offset !== undefined && params?.offset !== null
      ? Number(params.offset)
      : undefined;
  const limitVal =
    params?.limit !== undefined && params?.limit !== null
      ? Number(params.limit)
      : undefined;
  const startLine =
    offsetVal !== undefined && !isNaN(offsetVal)
      ? Math.max(1, offsetVal)
      : limitVal !== undefined && !isNaN(limitVal)
      ? 1
      : undefined;
  const endLine =
    startLine !== undefined && limitVal !== undefined && !isNaN(limitVal)
      ? startLine + limitVal - 1
      : undefined;

  let lineRangeBadge: string | null = null;
  if (startLine !== undefined && endLine !== undefined) {
    lineRangeBadge = `#L${startLine}-${endLine}`;
  } else if (startLine !== undefined) {
    lineRangeBadge = `#L${startLine}+`;
  } else if (tool.stdout) {
    const stdoutLines = tool.stdout.split('\n').filter((l: string) => l.trim().length > 0).length;
    lineRangeBadge = `· ${stdoutLines} lines`;
  } else if (limitVal !== undefined) {
    lineRangeBadge = `· ${limitVal} lines`;
  }

  return {
    toolName,
    params,
    isFileOp,
    isReadOp,
    isEditTool,
    isDiffTool,
    rawFilePath,
    normalizedPath,
    fileName,
    dirPath,
    oldString,
    newString,
    additions,
    deletions,
    opLabel,
    ext,
    lineRangeBadge,
    startLine,
    isFailed,
  };
}
