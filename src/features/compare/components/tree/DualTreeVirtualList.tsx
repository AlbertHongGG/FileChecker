import React from 'react';
import { FlattenedRow, PlannedAction, TreeNode, DirectoryActionType } from '../../../../types';
import { DualTreeRow } from './DualTreeRow';

interface DualTreeVirtualListProps {
  scrollContainerRef: React.RefObject<HTMLDivElement | null>;
  onScroll: (e: React.UIEvent<HTMLDivElement>) => void;
  onlyDiffs: boolean;
  rowsCount: number;
  visibleRows: FlattenedRow[];
  totalHeight: number;
  paddingTop: number;
  actionMap: Map<string, PlannedAction>;
  onToggleExpand: (path: string) => void;
  onSetAction: (path: string, act: PlannedAction) => void;
  onSetDirectoryAction?: (
    node: TreeNode,
    actionType: DirectoryActionType
  ) => void;
  onOpenDiff: (node: TreeNode) => void;
}

export const DualTreeVirtualList: React.FC<DualTreeVirtualListProps> = ({
  scrollContainerRef,
  onScroll,
  onlyDiffs,
  rowsCount,
  visibleRows,
  totalHeight,
  paddingTop,
  actionMap,
  onToggleExpand,
  onSetAction,
  onSetDirectoryAction,
  onOpenDiff,
}) => {
  return (
    <div
      ref={scrollContainerRef}
      onScroll={onScroll}
      style={{
        flex: 1,
        overflowY: 'auto',
        overflowX: 'hidden',
        position: 'relative',
      }}
    >
      {rowsCount === 0 ? (
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            height: '100%',
            color: 'var(--text-tertiary)',
            fontSize: '13px',
          }}
        >
          {onlyDiffs ? 'Identical (0 differences)' : 'Empty directory'}
        </div>
      ) : (
        <div style={{ height: `${totalHeight}px`, position: 'relative' }}>
          <div style={{ transform: `translateY(${paddingTop}px)` }}>
            {visibleRows.map((row) => (
              <DualTreeRow
                key={row.node.relative_path}
                row={row}
                action={actionMap.get(row.node.relative_path) || 'skip'}
                actionMap={actionMap}
                onToggleExpand={() => onToggleExpand(row.node.relative_path)}
                onSetAction={(act) => onSetAction(row.node.relative_path, act)}
                onSetDirectoryAction={
                  onSetDirectoryAction
                    ? (actionType) => onSetDirectoryAction(row.node, actionType)
                    : undefined
                }
                onOpenDiff={() => onOpenDiff(row.node)}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  );
};
