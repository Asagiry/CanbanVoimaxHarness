import { ToolExecution } from '../../types';
import CompactContextToolCard from './tools/CompactContextToolCard';
import SubagentToolCard from './tools/SubagentToolCard';
import FileOpsToolCard from './tools/FileOpsToolCard';
import SearchToolCard from './tools/SearchToolCard';
import CommandToolCard from './tools/CommandToolCard';
import WebToolCard from './tools/WebToolCard';
import GenericToolCard from './tools/GenericToolCard';

export interface ToolExecutionCardProps {
  tool: ToolExecution;
  onRerun?: (tool: ToolExecution) => void;
  isLatest?: boolean;
}

export default function ToolExecutionCard({ tool, onRerun, isLatest }: ToolExecutionCardProps) {
  if (!tool) return null;

  const toolName = tool.toolName ?? 'tool';

  switch (toolName) {
    case 'compact_context':
      return <CompactContextToolCard tool={tool} onRerun={onRerun} isLatest={isLatest} />;

    case 'subagent':
    case 'subagent_bg':
    case 'collect_subagent':
    case 'subagent_list':
    case 'list_subagents':
      return <SubagentToolCard tool={tool} onRerun={onRerun} isLatest={isLatest} />;

    case 'read':
    case 'read_file':
    case 'fetch_file':
    case 'write':
    case 'write_file':
    case 'edit':
    case 'edit_file':
    case 'str_replace':
      return <FileOpsToolCard tool={tool} onRerun={onRerun} isLatest={isLatest} />;

    case 'glob':
    case 'grep':
      return <SearchToolCard tool={tool} onRerun={onRerun} isLatest={isLatest} />;

    case 'execute_command':
    case 'bash':
    case 'pwsh':
      return <CommandToolCard tool={tool} onRerun={onRerun} isLatest={isLatest} />;

    case 'web_search':
    case 'web_fetch':
      return <WebToolCard tool={tool} onRerun={onRerun} isLatest={isLatest} />;

    default:
      return <GenericToolCard tool={tool} onRerun={onRerun} isLatest={isLatest} />;
  }
}
