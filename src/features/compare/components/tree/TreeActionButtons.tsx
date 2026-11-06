import React from 'react';
import {
  Check,
  ArrowRight,
  ArrowLeft as ArrowLeftIcon,
  Trash2,
  Minus,
  FileCode,
} from 'lucide-react';
import { TreeNode, PlannedAction } from '../../../../types';
import { Tooltip } from '../../../../components/common/Tooltip';

interface TreeActionButtonsProps {
  node: TreeNode;
  action: PlannedAction;
  existsInA: boolean;
  existsInB: boolean;
  onSetAction: (act: PlannedAction) => void;
  onOpenDiff: () => void;
}

export const TreeActionButtons: React.FC<TreeActionButtonsProps> = ({
  node,
  action,
  existsInA,
  existsInB,
  onSetAction,
  onOpenDiff,
}) => {
  if (node.is_dir) {
    return <span style={{ width: 14 }} />;
  }

  if (node.diff_kind.type === 'Identical') {
    return <Check size={12} style={{ color: 'var(--status-success)', opacity: 0.6 }} />;
  }

  const handleDelete = () => {
    if (existsInA && !existsInB) {
      onSetAction(action === 'delete_a' ? 'skip' : 'delete_a');
    } else if (existsInB && !existsInA) {
      onSetAction(action === 'delete_b' ? 'skip' : 'delete_b');
    } else {
      onSetAction(action === 'delete_b' ? 'skip' : 'delete_b');
    }
  };

  return (
    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '3px' }}>
      <Tooltip content="Copy A ➔ B" position="top">
        <button
          className="icon-btn"
          onClick={() => onSetAction(action === 'copy_a_to_b' ? 'skip' : 'copy_a_to_b')}
          style={{
            width: '24px',
            height: '24px',
            background: action === 'copy_a_to_b' ? 'var(--status-info)' : 'transparent',
            color: action === 'copy_a_to_b' ? '#ffffff' : 'var(--text-secondary)',
          }}
        >
          <ArrowRight size={12} />
        </button>
      </Tooltip>

      <Tooltip content="Copy B ⬅ A" position="top">
        <button
          className="icon-btn"
          onClick={() => onSetAction(action === 'copy_b_to_a' ? 'skip' : 'copy_b_to_a')}
          style={{
            width: '24px',
            height: '24px',
            background: action === 'copy_b_to_a' ? 'var(--status-info)' : 'transparent',
            color: action === 'copy_b_to_a' ? '#ffffff' : 'var(--text-secondary)',
          }}
        >
          <ArrowLeftIcon size={12} />
        </button>
      </Tooltip>

      <Tooltip content="Delete" position="top">
        <button
          className="icon-btn danger"
          onClick={handleDelete}
          style={{
            width: '24px',
            height: '24px',
            background:
              action === 'delete_a' || action === 'delete_b'
                ? 'var(--status-danger)'
                : 'transparent',
            color:
              action === 'delete_a' || action === 'delete_b'
                ? '#ffffff'
                : 'var(--text-secondary)',
          }}
        >
          <Trash2 size={12} />
        </button>
      </Tooltip>

      <Tooltip content="Skip" position="top">
        <button
          className="icon-btn"
          onClick={() => onSetAction('skip')}
          style={{
            width: '24px',
            height: '24px',
            color: action === 'skip' ? 'var(--text-tertiary)' : 'var(--text-secondary)',
          }}
        >
          <Minus size={12} />
        </button>
      </Tooltip>

      {node.diff_kind.type === 'Modified' && (
        <Tooltip content="Inspect Diff" position="top">
          <button
            className="icon-btn"
            onClick={onOpenDiff}
            style={{ width: '24px', height: '24px', color: 'var(--status-info)' }}
          >
            <FileCode size={13} />
          </button>
        </Tooltip>
      )}
    </div>
  );
};
