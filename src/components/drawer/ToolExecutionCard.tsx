import { ToolExecution } from '../../types';
import CompactContextToolCard from './tools/CompactContextToolCard';
import SubagentToolCard from './tools/SubagentToolCard';
import FileOpsToolCard from './tools/FileOpsToolCard';
import SearchToolCard from './tools/SearchToolCard';
import CommandToolCard from './tools/CommandToolCard';
import WebToolCard from './tools/WebToolCard';
import TodoToolCard from './tools/TodoToolCard';
import GenericToolCard from './tools/GenericToolCard';

export { BatchBadge, isToolSkipped, StatusIndicator } from './tools/shared';

export interface ToolExecutionCardProps {
  tool: ToolExecution;
  onRerun?: (tool: ToolExecution) => void;
  isLatest?: boolean;
}

export default function ToolExecutionCard({ tool, onRerun, isLatest }: ToolExecutionCardProps) {
  if (!tool) return null;

  const toolName = tool.toolName ?? 'tool';

  let card: React.ReactNode;
  switch (toolName) {
    case 'todo_write':
    case 'write_todos':
      card = <TodoToolCard tool={tool} onRerun={onRerun} isLatest={isLatest} />;
      break;

    case 'compact_context':
      card = <CompactContextToolCard tool={tool} onRerun={onRerun} isLatest={isLatest} />;
      break;

    case 'subagent':
    case 'subagent_bg':
    case 'collect_subagent':
    case 'subagent_list':
    case 'list_subagents':
      card = <SubagentToolCard tool={tool} onRerun={onRerun} isLatest={isLatest} />;
      break;

    case 'read':
    case 'read_file':
    case 'fetch_file':
    case 'write':
    case 'write_file':
    case 'edit':
    case 'edit_file':
    case 'str_replace':
      card = <FileOpsToolCard tool={tool} onRerun={onRerun} isLatest={isLatest} />;
      break;

    case 'glob':
    case 'grep':
      card = <SearchToolCard tool={tool} onRerun={onRerun} isLatest={isLatest} />;
      break;

    case 'execute_command':
    case 'bash':
    case 'pwsh':
      card = <CommandToolCard tool={tool} onRerun={onRerun} isLatest={isLatest} />;
      break;

    case 'web_search':
    case 'web_fetch':
      card = <WebToolCard tool={tool} onRerun={onRerun} isLatest={isLatest} />;
      break;

    default:
      card = <GenericToolCard tool={tool} onRerun={onRerun} isLatest={isLatest} />;
      break;
  }

  if (tool.status === 'queued') {
    return <div className="opacity-75 dark:opacity-70 transition-opacity">{card}</div>;
  }

  return card;
}
