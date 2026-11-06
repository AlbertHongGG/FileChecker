import React from 'react';
import { FlattenedRow, PlannedAction } from '../../../../types';
import { TreeSideNode } from './TreeSideNode';
import { TreeActionButtons } from './TreeActionButtons';
import { ROW_HEIGHT } from '../../hooks/useVirtualScroll';

interface DualTreeRowProps {
  row: FlattenedRow;
  action: PlannedAction;
  onToggleExpand: () => void;
  onSetAction: (act: PlannedAction) => void;
  onOpenDiff: () => void;
}

export const DualTreeRow: React.FC<DualTreeRowProps> = ({
  row,
  action,
  onToggleExpand,
  onSetAction,
  onOpenDiff,
}) => {
  const { node, depth, hasChildren, isExpanded } = row;
  const existsInA = Boolean(node.meta_a);
  const existsInB = Boolean(node.meta_b);

  return (
    <div
      style={{
        display: 'grid',
        gridTemplateColumns: '1fr 140px 1fr',
        height: `${ROW_HEIGHT}px`,
        borderBottom: '1px solid var(--border-subtle)',
        alignItems: 'center',
        fontSize: '12px',
        boxSizing: 'border-box',
      }}
      onMouseEnter={(e) => {
        e.currentTarget.style.backgroundColor = 'var(--bg-surface-hover)';
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.backgroundColor = 'transparent';
      }}
    >
      {/* Side A */}
      <TreeSideNode
        side="A"
        node={node}
        depth={depth}
        hasChildren={hasChildren}
        isExpanded={isExpanded}
        action={action}
        onToggleExpand={onToggleExpand}
      />

      {/* Middle Action Buttons */}
      <TreeActionButtons
        node={node}
        action={action}
        existsInA={existsInA}
        existsInB={existsInB}
        onSetAction={onSetAction}
        onOpenDiff={onOpenDiff}
      />

      {/* Side B */}
      <TreeSideNode
        side="B"
        node={node}
        depth={depth}
        hasChildren={hasChildren}
        isExpanded={isExpanded}
        action={action}
      />
    </div>
  );
};
