import React from 'react';
import { ChevronRight, ChevronDown, Folder, FileText } from 'lucide-react';
import { TreeNode, PlannedAction } from '../../../../types';
import { Tooltip } from '../../../../components/common/Tooltip';
import { formatFileSize, formatTimestamp } from '../../utils/formatters';

interface TreeSideNodeProps {
  side: 'A' | 'B';
  node: TreeNode;
  depth: number;
  hasChildren: boolean;
  isExpanded: boolean;
  action: PlannedAction;
  onToggleExpand?: () => void;
}

export const TreeSideNode: React.FC<TreeSideNodeProps> = ({
  side,
  node,
  depth,
  hasChildren,
  isExpanded,
  action,
  onToggleExpand,
}) => {
  const isSideA = side === 'A';
  const meta = isSideA ? node.meta_a : node.meta_b;
  const otherMeta = isSideA ? node.meta_b : node.meta_a;
  const exists = Boolean(meta);
  const existsOther = Boolean(otherMeta);

  // Transfer & Ghost states: directly resolved via node's own action (O(1))
  const isGhost = !exists && (isSideA ? action === 'copy_b_to_a' : action === 'copy_a_to_b');
  const isDeleted =
    exists &&
    (action === 'delete_both' ||
      (isSideA ? action === 'delete_a' : action === 'delete_b'));
  const isTransferredOut = exists && (isSideA ? action === 'copy_a_to_b' : action === 'copy_b_to_a');
  const isOverwritten = exists && (isSideA ? action === 'copy_b_to_a' : action === 'copy_a_to_b');

  return (
    <div
      style={{
        paddingLeft: `${depth * 18 + 8}px`,
        display: 'flex',
        alignItems: 'center',
        gap: '6px',
        overflow: 'hidden',
        minWidth: 0,
        opacity: isDeleted ? 0.45 : isGhost ? 0.75 : !exists ? 0.35 : 1,
        textDecoration: isDeleted ? 'line-through' : 'none',
      }}
    >
      {/* Expander Arrow on both sides whenever this tree row has children */}
      {hasChildren ? (
        <button
          onClick={onToggleExpand}
          style={{
            background: 'transparent',
            border: 'none',
            cursor: 'pointer',
            padding: 0,
            color: 'var(--text-tertiary)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            width: '14px',
            height: '14px',
          }}
          title={isExpanded ? 'Collapse Folder' : 'Expand Folder'}
        >
          {isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
        </button>
      ) : (
        <span style={{ width: 14 }} />
      )}

      {/* Node Icon */}
      {node.is_dir ? (
        <Folder
          size={14}
          style={{
            color: exists ? 'var(--text-tertiary)' : 'var(--text-tertiary)',
            opacity: exists ? 1 : 0.4,
            flexShrink: 0,
          }}
        />
      ) : (
        <FileText
          size={14}
          style={{
            color: exists ? 'var(--text-secondary)' : 'var(--text-tertiary)',
            opacity: exists ? 1 : 0.4,
            flexShrink: 0,
          }}
        />
      )}

      {/* Name / Ghost Name / Disabled Name (No "—" placeholder, normal display but disabled) */}
      {exists ? (
        <span
          style={{
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            color: isDeleted ? 'var(--status-danger)' : 'var(--text-primary)',
            fontWeight: exists && !existsOther ? 600 : 400,
          }}
        >
          {node.name}
        </span>
      ) : isGhost ? (
        <span
          style={{
            fontStyle: 'italic',
            color: 'var(--status-success)',
            borderBottom: '1px dashed var(--status-success)',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}
        >
          + {node.name}
        </span>
      ) : (
        /* When item does not exist on this side: display name normally with disabled/dimmed styling */
        <span
          style={{
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            color: 'var(--text-tertiary)',
            userSelect: 'none',
          }}
        >
          {node.name}
        </span>
      )}

      {/* Transfer Badges */}
      {isTransferredOut && (
        <span
          style={{
            fontSize: '10px',
            padding: '1px 4px',
            borderRadius: '4px',
            background: 'var(--status-info-light)',
            color: 'var(--status-info)',
            flexShrink: 0,
          }}
        >
          {isSideA ? '➔ B' : '⬅ A'}
        </span>
      )}

      {isOverwritten && (
        <span
          style={{
            fontSize: '10px',
            padding: '1px 4px',
            borderRadius: '4px',
            background: 'var(--status-warning-light)',
            color: 'var(--status-warning)',
            flexShrink: 0,
          }}
        >
          ⚠ Overwrite
        </span>
      )}

      {isDeleted && (
        <span
          style={{
            fontSize: '10px',
            padding: '1px 4px',
            borderRadius: '4px',
            background: 'var(--status-danger-light)',
            color: 'var(--status-danger)',
            flexShrink: 0,
          }}
        >
          ✕ Del
        </span>
      )}

      {/* File Size & Timestamp Badge */}
      {exists && !node.is_dir && (
        <Tooltip
          content={`Modified: ${formatTimestamp(meta?.modified_timestamp)}`}
          position="top"
        >
          <span
            style={{
              fontSize: '11px',
              color: 'var(--text-tertiary)',
              marginLeft: 'auto',
              paddingRight: '8px',
              flexShrink: 0,
              cursor: 'default',
            }}
          >
            {formatFileSize(meta?.size)}
          </span>
        </Tooltip>
      )}
    </div>
  );
};
