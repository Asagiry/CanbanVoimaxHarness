import { describe, it, expect } from 'vitest';
import { renderToString } from 'react-dom/server';
import { HookIcon } from '../HookIcon';
import { hookMomentOf } from '../../cortex/types';
import type { CortexAgentFile } from '../../../types';

describe('hookMomentOf resolver', () => {
  it('resolves hook moments from CortexAgentFile object fields (hookMoment and id)', () => {
    const fileWithMoment: CortexAgentFile = {
      id: 'custom-id',
      name: 'gatekeeper_sensor.bat',
      stageId: 'review',
      type: 'hook',
      hookMoment: 'on_enter',
      path: '.cvh/hooks/gatekeeper_sensor.bat',
      content: '',
    };
    expect(hookMomentOf(fileWithMoment)).toBe('on_enter');

    const fileWithIdEnter: CortexAgentFile = {
      id: 'review-hook-on_enter',
      name: 'gatekeeper_sensor.bat',
      stageId: 'review',
      type: 'hook',
      path: '.cvh/hooks/gatekeeper_sensor.bat',
      content: '',
    };
    expect(hookMomentOf(fileWithIdEnter)).toBe('on_enter');

    const fileWithIdLeave: CortexAgentFile = {
      id: 'dev-hook-on_leave',
      name: 'cleanup.bat',
      stageId: 'dev',
      type: 'hook',
      path: '.cvh/hooks/cleanup.bat',
      content: '',
    };
    expect(hookMomentOf(fileWithIdLeave)).toBe('on_leave');

    const fileWithIdFinish: CortexAgentFile = {
      id: 'review-hook-on_finish',
      name: 'notify.bat',
      stageId: 'review',
      type: 'hook',
      path: '.cvh/hooks/notify.bat',
      content: '',
    };
    expect(hookMomentOf(fileWithIdFinish)).toBe('on_finish');

    const fileWithIdExit: CortexAgentFile = {
      id: 'test-hook-on_exit',
      name: 'teardown.bat',
      stageId: 'test',
      type: 'hook',
      path: '.cvh/hooks/teardown.bat',
      content: '',
    };
    expect(hookMomentOf(fileWithIdExit)).toBe('on_leave');
  });

  it('resolves hook moments from CortexAgentFile path fallback', () => {
    const fileWithPath: CortexAgentFile = {
      id: 'generic-hook-1',
      name: 'on_review_finish.bat',
      stageId: 'review',
      type: 'hook',
      path: '.cvh/agents/reviewer/hooks/on_review_finish.bat',
      content: '',
    };
    expect(hookMomentOf(fileWithPath)).toBe('on_finish');
  });

  it('resolves hook moments from string prefixes and infixes', () => {
    // on_enter variants
    expect(hookMomentOf('on_enter.bat')).toBe('on_enter');
    expect(hookMomentOf('on-enter.sh')).toBe('on_enter');
    expect(hookMomentOf('enter_dev.bat')).toBe('on_enter');
    expect(hookMomentOf('enter-build.ps1')).toBe('on_enter');
    expect(hookMomentOf('stage_on_enter.bat')).toBe('on_enter');
    expect(hookMomentOf('pre_enter_check.sh')).toBe('on_enter');
    expect(hookMomentOf('gate-enter.bat')).toBe('on_enter');
    expect(hookMomentOf('builtin:on_enter')).toBe('on_enter');

    // on_leave / on_exit variants
    expect(hookMomentOf('on_leave.bat')).toBe('on_leave');
    expect(hookMomentOf('on-leave.ps1')).toBe('on_leave');
    expect(hookMomentOf('leave_stage.sh')).toBe('on_leave');
    expect(hookMomentOf('leave-task.bat')).toBe('on_leave');
    expect(hookMomentOf('post_leave_cleanup.sh')).toBe('on_leave');
    expect(hookMomentOf('on_exit.sh')).toBe('on_leave');
    expect(hookMomentOf('on-exit.bat')).toBe('on_leave');
    expect(hookMomentOf('exit_runner.py')).toBe('on_leave');
    expect(hookMomentOf('task_exit.bat')).toBe('on_leave');
    expect(hookMomentOf('subtask-exit.bat')).toBe('on_leave');
    expect(hookMomentOf('builtin:on_leave')).toBe('on_leave');

    // on_finish variants
    expect(hookMomentOf('on_finish.bat')).toBe('on_finish');
    expect(hookMomentOf('on-finish.sh')).toBe('on_finish');
    expect(hookMomentOf('finish_pipeline.ps1')).toBe('on_finish');
    expect(hookMomentOf('finish-check.bat')).toBe('on_finish');
    expect(hookMomentOf('on_review_finish.bat')).toBe('on_finish');
    expect(hookMomentOf('step-finish.sh')).toBe('on_finish');
  });

  it('handles negative cases and empty inputs correctly', () => {
    expect(hookMomentOf(null)).toBeNull();
    expect(hookMomentOf(undefined)).toBeNull();
    expect(hookMomentOf('')).toBeNull();
    expect(hookMomentOf('   ')).toBeNull();
    expect(hookMomentOf('random_script.bat')).toBeNull();
    expect(hookMomentOf('gatekeeper_sensor.bat')).toBeNull();
    expect(hookMomentOf('center_element.js')).toBeNull();
    expect(hookMomentOf('clever_idea.py')).toBeNull();
  });
});

describe('HookIcon component', () => {
  it('renders ArrowRightToLine for on_enter moment', () => {
    const html = renderToString(<HookIcon moment="on_enter" />);
    expect(html).toContain('aria-label="on_enter hook"');
    expect(html).toContain('text-emerald-600');
    expect(html).toContain('dark:text-emerald-400');
  });

  it('renders ArrowRightFromLine for on_leave moment', () => {
    const html = renderToString(<HookIcon moment="on_leave" />);
    expect(html).toContain('aria-label="on_leave hook"');
    expect(html).toContain('text-amber-600');
    expect(html).toContain('dark:text-amber-400');
  });

  it('renders ArrowRightFromLine for on_finish moment', () => {
    const html = renderToString(<HookIcon moment="on_finish" />);
    expect(html).toContain('aria-label="on_finish hook"');
    expect(html).toContain('text-amber-600');
    expect(html).toContain('dark:text-amber-400');
  });

  it('renders ArrowRightFromLine for on_exit moment', () => {
    const html = renderToString(<HookIcon moment="on_exit" />);
    expect(html).toContain('aria-label="on_exit hook"');
    expect(html).toContain('text-amber-600');
    expect(html).toContain('dark:text-amber-400');
  });

  it('renders Zap fallback when moment is not recognized', () => {
    const html = renderToString(<HookIcon moment={null} />);
    expect(html).toContain('aria-label="hook"');
    expect(html).toContain('text-[#666666]');
  });

  it('resolves icon from file prop (on_enter)', () => {
    const file: CortexAgentFile = {
      id: 'review-hook-on_enter',
      name: 'gatekeeper_sensor.bat',
      stageId: 'review',
      type: 'hook',
      path: '.cvh/hooks/gatekeeper_sensor.bat',
      content: '',
    };
    const html = renderToString(<HookIcon file={file} />);
    expect(html).toContain('aria-label="on_enter hook"');
    expect(html).toContain('text-emerald-600');
  });

  it('resolves icon from file prop (on_finish)', () => {
    const file: CortexAgentFile = {
      id: 'review-hook-on_finish',
      name: 'on_review_finish.bat',
      stageId: 'review',
      type: 'hook',
      path: '.cvh/agents/reviewer/hooks/on_review_finish.bat',
      content: '',
    };
    const html = renderToString(<HookIcon file={file} />);
    expect(html).toContain('aria-label="on_finish hook"');
    expect(html).toContain('text-amber-600');
  });

  it('applies custom className correctly', () => {
    const html = renderToString(<HookIcon moment="on_enter" className="h-4 w-4 custom-hook-icon" />);
    expect(html).toContain('custom-hook-icon');
    expect(html).toContain('h-4 w-4');
  });
});
