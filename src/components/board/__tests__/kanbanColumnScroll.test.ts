import { describe, it, expect } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';

describe('Kanban Column Hover-Only 3px Scrollbar', () => {
  const indexCssPath = path.resolve(__dirname, '../../../index.css');
  const kanbanColumnPath = path.resolve(__dirname, '../KanbanColumn.tsx');

  const indexCss = fs.readFileSync(indexCssPath, 'utf-8');
  const kanbanColumnSource = fs.readFileSync(kanbanColumnPath, 'utf-8');

  describe('src/index.css scrollbar definitions', () => {
    it('defines .kanban-column-scroll with standard W3C properties', () => {
      expect(indexCss).toContain('.kanban-column-scroll');
      expect(indexCss).toContain('scrollbar-width: thin;');
      expect(indexCss).toContain('scrollbar-color: transparent transparent;');
    });

    it('sets 3px width and transparent track/thumb by default in WebKit', () => {
      expect(indexCss).toMatch(/\.kanban-column-scroll::-webkit-scrollbar\s*\{[^}]*width:\s*3px;/);
      expect(indexCss).toMatch(/\.kanban-column-scroll::-webkit-scrollbar\s*\{[^}]*height:\s*3px;/);
      expect(indexCss).toMatch(/\.kanban-column-scroll::-webkit-scrollbar-track\s*\{[^}]*background:\s*transparent;/);
      expect(indexCss).toMatch(/\.kanban-column-scroll::-webkit-scrollbar-thumb\s*\{[^}]*background:\s*transparent;/);
      expect(indexCss).toMatch(/\.kanban-column-scroll::-webkit-scrollbar-thumb\s*\{[^}]*border-radius:\s*9999px;/);
    });

    it('surfaces light theme thumb on hover and active states', () => {
      expect(indexCss).toContain('[data-kanban-column]:hover .kanban-column-scroll');
      expect(indexCss).toMatch(/\[data-kanban-column\]:hover\s+\.kanban-column-scroll::-webkit-scrollbar-thumb/);
      expect(indexCss).toContain('.kanban-column-scroll::-webkit-scrollbar-thumb:active');
      expect(indexCss).toContain('background: #d4d4d8;');
      expect(indexCss).toContain('background: #a1a1aa;');
    });

    it('surfaces dark theme thumb with obsidian palette on hover and active states', () => {
      expect(indexCss).toContain('.dark [data-kanban-column]:hover .kanban-column-scroll');
      expect(indexCss).toMatch(/\.dark\s+\[data-kanban-column\]:hover\s+\.kanban-column-scroll::-webkit-scrollbar-thumb/);
      expect(indexCss).toContain('.dark .kanban-column-scroll::-webkit-scrollbar-thumb:active');
      expect(indexCss).toContain('background: #30363d;');
      expect(indexCss).toContain('background: #484f58;');
    });
  });

  describe('src/components/board/KanbanColumn.tsx integration', () => {
    it('applies kanban-column-scroll class to droppable card container', () => {
      expect(kanbanColumnSource).toContain('kanban-column-scroll');
      expect(kanbanColumnSource).toMatch(/className=\{`[^`]*overflow-y-auto[^`]*kanban-column-scroll/);
    });

    it('has data-kanban-column attribute on the root column element for hover delegation', () => {
      expect(kanbanColumnSource).toContain('data-kanban-column={column.id}');
    });
  });
});
