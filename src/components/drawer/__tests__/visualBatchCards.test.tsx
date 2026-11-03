import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderToString } from 'react-dom/server';
import {
  isToolSkipped,
  BatchBadge,
  StatusIndicator,
} from '../tools/shared';
import ToolExecutionCard from '../ToolExecutionCard';
import DrawerActivityStream from '../DrawerActivityStream';
import { ToolExecution, KanbanTask } from '../../../types';
import type { DrawerActivityItem } from '../../../stores/drawerStore';
import { useLanguageStore } from '../../../stores/languageStore';

// Mock Tauri invoke and store APIs
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue([]),
}));

vi.mock('@tauri-apps/plugin-notification', () => ({
  isPermissionGranted: vi.fn().mockResolvedValue(true),
  requestPermission: vi.fn().mockResolvedValue('granted'),
  sendNotification: vi.fn(),
}));

vi.mock('../../../services/tauriEventGateway', () => ({
  tauriEventGateway: {
    onAgentEvent: vi.fn().mockResolvedValue(() => {}),
  },
}));

describe('Visual Batch Tool Execution: isToolSkipped', () => {
  it('identifies tools skipped due to fail-fast via stdout', () => {
    const skippedTool: ToolExecution = {
      id: 'call_1',
      toolName: 'edit_file',
      timestamp: '12:00:00',
      durationMs: 0,
      status: 'failed',
      params: { file_path: 'test.rs' },
      stdout: "Skipped: previous tool 'edit_file' in this batch failed",
    };
    expect(isToolSkipped(skippedTool)).toBe(true);
  });

  it('identifies tools skipped due to fail-fast via stderr', () => {
    const skippedTool: ToolExecution = {
      id: 'call_2',
      toolName: 'execute_command',
      timestamp: '12:00:01',
      durationMs: 0,
      status: 'failed',
      params: { command: 'cargo test' },
      stderr: "Skipped: previous tool 'read_file' in this batch failed",
    };
    expect(isToolSkipped(skippedTool)).toBe(true);
  });

  it('returns false for normal successful or failed tools', () => {
    const successTool: ToolExecution = {
      id: 'call_3',
      toolName: 'read_file',
      timestamp: '12:00:02',
      durationMs: 50,
      status: 'success',
      params: {},
      stdout: 'file contents',
    };
    expect(isToolSkipped(successTool)).toBe(false);

    const regularFailedTool: ToolExecution = {
      id: 'call_4',
      toolName: 'edit_file',
      timestamp: '12:00:03',
      durationMs: 10,
      status: 'failed',
      params: {},
      stderr: 'File not found on disk',
    };
    expect(isToolSkipped(regularFailedTool)).toBe(false);
  });
});

describe('Visual Batch Tool Execution: BatchBadge', () => {
  beforeEach(() => {
    useLanguageStore.setState({ language: 'en' });
  });

  it('returns null when tool has no batch or total <= 1', () => {
    const noBatchTool: ToolExecution = {
      id: 'call_1',
      toolName: 'read_file',
      timestamp: '12:00:00',
      durationMs: 10,
      status: 'success',
      params: {},
    };
    expect(renderToString(<BatchBadge tool={noBatchTool} />)).toBe('');

    const singleBatchTool: ToolExecution = {
      ...noBatchTool,
      batch: { index: 1, total: 1 },
    };
    expect(renderToString(<BatchBadge tool={singleBatchTool} />)).toBe('');
  });

  it('renders distinct batch step pill in English', () => {
    useLanguageStore.getState().setLanguage('en');
    const batchTool: ToolExecution = {
      id: 'call_1',
      toolName: 'edit_file',
      timestamp: '12:00:00',
      durationMs: 10,
      status: 'running',
      params: {},
      batch: { index: 2, total: 4 },
    };

    const html = renderToString(<BatchBadge tool={batchTool} />);
    expect(html).toContain('🔗');
    expect(html).toContain('Batch 2/4');
    expect(html).toContain('data-testid="batch-step-badge"');
  });

  it('renders distinct batch step pill in Russian', () => {
    useLanguageStore.getState().setLanguage('ru');
    const batchTool: ToolExecution = {
      id: 'call_1',
      toolName: 'edit_file',
      timestamp: '12:00:00',
      durationMs: 10,
      status: 'running',
      params: {},
      batch: { index: 2, total: 4 },
    };

    const html = renderToString(<BatchBadge tool={batchTool} />);
    expect(html).toContain('🔗');
    expect(html).toContain('Пакет 2/4');
    expect(html).toContain('data-testid="batch-step-badge"');
  });
});

describe('Visual Batch Tool Execution: StatusIndicator', () => {
  beforeEach(() => {
    useLanguageStore.setState({ language: 'ru' });
  });

  it('renders muted amber skipped status pill when tool was skipped', () => {
    const skippedTool: ToolExecution = {
      id: 'call_skip',
      toolName: 'execute_command',
      timestamp: '12:00:00',
      durationMs: 0,
      status: 'failed',
      params: {},
      stdout: "Skipped: previous tool 'edit_file' in this batch failed",
      batch: { index: 2, total: 3 },
    };

    const html = renderToString(<StatusIndicator tool={skippedTool} />);
    expect(html).toContain('data-testid="batch-skipped-badge"');
    expect(html).toContain('🚫');
    expect(htmlRuOrEn(html, 'Пропущено из-за сбоя в пакете', 'Skipped due to batch failure')).toBe(true);
    // Should NOT render generic red X icon
    expect(html).not.toContain('text-rose-600');
  });

  it('renders Check icon on normal success', () => {
    const successTool: ToolExecution = {
      id: 'call_ok',
      toolName: 'edit_file',
      timestamp: '12:00:00',
      durationMs: 50,
      status: 'success',
      params: {},
    };

    const html = renderToString(<StatusIndicator tool={successTool} />);
    expect(html).toContain('text-emerald-600');
    expect(html).not.toContain('🚫');
  });

  it('renders red X icon on regular failure without skip note', () => {
    const failedTool: ToolExecution = {
      id: 'call_fail',
      toolName: 'edit_file',
      timestamp: '12:00:00',
      durationMs: 50,
      status: 'failed',
      params: {},
      stderr: 'Failed to find old_string in file',
    };

    const html = renderToString(<StatusIndicator tool={failedTool} />);
    expect(html).toContain('text-rose-600');
    expect(html).not.toContain('🚫');
  });
});

describe('Visual Batch Tool Execution: ToolExecutionCard Integration', () => {
  it('renders batch pill in header of FileOpsToolCard and CommandToolCard', () => {
    useLanguageStore.setState({ language: 'en' });

    const fileTool: ToolExecution = {
      id: 'call_file',
      toolName: 'edit_file',
      timestamp: '12:00:00',
      durationMs: 30,
      status: 'success',
      params: { file_path: 'src/main.rs', old_string: 'a', new_string: 'b' },
      batch: { index: 1, total: 3 },
    };

    const fileHtml = renderToString(<ToolExecutionCard tool={fileTool} />);
    expect(fileHtml).toContain('data-testid="batch-step-badge"');
    expect(fileHtml).toContain('Batch 1/3');

    const cmdTool: ToolExecution = {
      id: 'call_cmd',
      toolName: 'execute_command',
      timestamp: '12:00:01',
      durationMs: 0,
      status: 'failed',
      params: { command: 'cargo test' },
      stdout: "Skipped: previous tool 'edit_file' in this batch failed",
      batch: { index: 2, total: 3 },
    };

    const cmdHtml = renderToString(<ToolExecutionCard tool={cmdTool} />);
    expect(cmdHtml).toContain('data-testid="batch-step-badge"');
    expect(cmdHtml).toContain('Batch 2/3');
    expect(cmdHtml).toContain('data-testid="batch-skipped-badge"');
    expect(cmdHtml).toContain('🚫');
  });
});

describe('Visual Batch Tool Execution: DrawerActivityStream Chain Connector', () => {
  const dummyTask: KanbanTask = {
    id: 'TASK-001',
    title: 'Test Batch Chain',
    stageId: 'dev',
    status: 'running',
    persona: 'developer',
    priority: 'medium',
    activeStageDuration: 0,
    stageDurations: {},
    totalElapsed: 0,
    isTimerRunning: true,
    returnsCount: 0,
    verdict: 'NONE',
  };

  it('renders pipeline chain connector between consecutive batch cards', () => {
    const activities: DrawerActivityItem[] = [
      {
        id: 'act-1',
        type: 'tool',
        timestamp: '12:00:00',
        tool: {
          id: 'call_1',
          toolName: 'edit_file',
          timestamp: '12:00:00',
          durationMs: 20,
          status: 'success',
          params: { file_path: 'a.rs' },
          batch: { index: 1, total: 3 },
        },
      },
      {
        id: 'act-2',
        type: 'tool',
        timestamp: '12:00:01',
        tool: {
          id: 'call_2',
          toolName: 'edit_file',
          timestamp: '12:00:01',
          durationMs: 25,
          status: 'success',
          params: { file_path: 'b.rs' },
          batch: { index: 2, total: 3 },
        },
      },
      {
        id: 'act-3',
        type: 'tool',
        timestamp: '12:00:02',
        tool: {
          id: 'call_3',
          toolName: 'execute_command',
          timestamp: '12:00:02',
          durationMs: 100,
          status: 'success',
          params: { command: 'cargo test' },
          batch: { index: 3, total: 3 },
        },
      },
    ];

    const html = renderToString(
      <DrawerActivityStream
        task={dummyTask}
        unifiedActivities={activities}
        isRunning={false}
        runError={null}
        onRunAgent={() => {}}
        onClearRunError={() => {}}
      />
    );

    // Expect exactly 2 connectors (between 1->2 and 2->3, none after 3)
    const matches = html.match(/data-testid="batch-chain-connector"/g) || [];
    expect(matches.length).toBe(2);
    expect(html).toContain('border-dashed');
  });

  it('does not render chain connector for isolated or terminal tools', () => {
    const activities: DrawerActivityItem[] = [
      {
        id: 'act-single',
        type: 'tool',
        timestamp: '12:00:00',
        tool: {
          id: 'call_single',
          toolName: 'read_file',
          timestamp: '12:00:00',
          durationMs: 15,
          status: 'success',
          params: {},
        },
      },
    ];

    const html = renderToString(
      <DrawerActivityStream
        task={dummyTask}
        unifiedActivities={activities}
        isRunning={false}
        runError={null}
        onRunAgent={() => {}}
        onClearRunError={() => {}}
      />
    );

    expect(html).not.toContain('data-testid="batch-chain-connector"');
  });
});

function htmlRuOrEn(html: string, ru: string, en: string): boolean {
  return html.includes(ru) || html.includes(en);
}
