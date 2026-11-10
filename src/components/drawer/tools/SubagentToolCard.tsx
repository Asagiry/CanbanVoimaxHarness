import { ToolCardProps } from './shared';
import {
  CollectSubagentExecutionCard,
  SubagentListExecutionCard,
  SingleSubagentExecutionCard,
} from './subagent';

export * from './subagent';

export default function SubagentToolCard({ tool, isExpanded, onToggleExpanded }: ToolCardProps) {
  const toolName = tool.toolName ?? 'subagent';

  if (toolName === 'collect_subagent') {
    return <CollectSubagentExecutionCard tool={tool} isExpanded={isExpanded} onToggleExpanded={onToggleExpanded} />;
  }

  if (toolName === 'subagent_list' || toolName === 'list_subagents') {
    return <SubagentListExecutionCard tool={tool} isExpanded={isExpanded} onToggleExpanded={onToggleExpanded} />;
  }

  return <SingleSubagentExecutionCard tool={tool} isExpanded={isExpanded} onToggleExpanded={onToggleExpanded} />;
}
