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
  actionMap?: Map<string, PlannedAction>;
  onSetAction: (act: PlannedAction) => void;
  onSetDirectoryAction?: (actionType: 'copy_a_to_b' | 'copy_b_to_a' | 'delete' | 'skip') => void;
  onOpenDiff: () => void;
}

export const TreeActionButtons: React.FC<TreeActionButtonsProps> = ({
  node,
  action,
  existsInA,
  existsInB,
  actionMap,
  onSetAction,
  onSetDirectoryAction,
  onOpenDiff,
}) => {
  // ─── DIRECTORY ACTION BUTTONS ────────────────────────────
  if (node.is_dir) {
    if (node.diff_kind.type === 'Identical') {
      return <Check size={12} style={{ color: 'var(--status-success)', opacity: 0.6 }} />;
    }

    const canCopyAtoB = Boolean(node.canCopyAtoB);
    const canCopyBtoA = Boolean(node.canCopyBtoA);
    const canDelete = Boolean(node.canDelete);
    const hasModifiable = Boolean(node.modifiableFiles && node.modifiableFiles.length > 0);

    // Compute active states for folder action buttons based on all subtree entries
    const descendants = node.descendantEntries || [];

    const eligibleAtoB = descendants.filter((d) => d.diffType === 'OnlyInA' || d.diffType === 'Modified');
    const isCopyAtoBActive =
      eligibleAtoB.length > 0 &&
      Boolean(actionMap) &&
      eligibleAtoB.every((d) => actionMap?.get(d.path) === 'copy_a_to_b');

    const eligibleBtoA = descendants.filter((d) => d.diffType === 'OnlyInB' || d.diffType === 'Modified');
    const isCopyBtoAActive =
      eligibleBtoA.length > 0 &&
      Boolean(actionMap) &&
      eligibleBtoA.every((d) => actionMap?.get(d.path) === 'copy_b_to_a');

    const eligibleDelete = descendants.filter((d) => d.diffType !== 'Identical');
    const isDeleteActive =
      eligibleDelete.length > 0 &&
      Boolean(actionMap) &&
      eligibleDelete.every((d) => {
        const act = actionMap?.get(d.path);
        return act === 'delete_a' || act === 'delete_b' || act === 'delete_both';
      });

    const isPureEmpty = descendants.length <= 1;

    const tooltipAtoB = !canCopyAtoB
      ? isPureEmpty ? 'Folder does not exist in A' : 'No files to copy to B'
      : isCopyAtoBActive
      ? 'Undo Copy Folder to B'
      : isPureEmpty ? 'Copy empty folder to Directory B' : 'Copy Folder contents to Directory B';

    const tooltipBtoA = !canCopyBtoA
      ? isPureEmpty ? 'Folder does not exist in B' : 'No files to copy to A'
      : isCopyBtoAActive
      ? 'Undo Copy Folder to A'
      : isPureEmpty ? 'Copy empty folder to Directory A' : 'Copy Folder contents to Directory A';

    const tooltipDelete = !canDelete
      ? 'No folder to delete'
      : isDeleteActive
      ? 'Undo Delete Folder'
      : isPureEmpty ? 'Delete empty folder' : 'Delete Folder contents';

    const tooltipDiff = hasModifiable
      ? `Inspect Diff for ${node.modifiableFiles?.length} modified file(s)`
      : 'No modified files to inspect in folder';

    return (
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '3px' }}>
        {/* Copy A to B Button */}
        <Tooltip content={tooltipAtoB} position="top">
          <button
            className="icon-btn"
            disabled={!canCopyAtoB}
            onClick={() => canCopyAtoB && onSetDirectoryAction && onSetDirectoryAction('copy_a_to_b')}
            style={{
              width: '24px',
              height: '24px',
              opacity: canCopyAtoB ? 1 : 0.25,
              cursor: canCopyAtoB ? 'pointer' : 'not-allowed',
              background: isCopyAtoBActive ? 'var(--status-info)' : 'transparent',
              color: isCopyAtoBActive ? '#ffffff' : 'var(--text-secondary)',
            }}
          >
            <ArrowRight size={12} />
          </button>
        </Tooltip>

        {/* Copy B to A Button */}
        <Tooltip content={tooltipBtoA} position="top">
          <button
            className="icon-btn"
            disabled={!canCopyBtoA}
            onClick={() => canCopyBtoA && onSetDirectoryAction && onSetDirectoryAction('copy_b_to_a')}
            style={{
              width: '24px',
              height: '24px',
              opacity: canCopyBtoA ? 1 : 0.25,
              cursor: canCopyBtoA ? 'pointer' : 'not-allowed',
              background: isCopyBtoAActive ? 'var(--status-info)' : 'transparent',
              color: isCopyBtoAActive ? '#ffffff' : 'var(--text-secondary)',
            }}
          >
            <ArrowLeftIcon size={12} />
          </button>
        </Tooltip>

        {/* Delete Button */}
        <Tooltip content={tooltipDelete} position="top">
          <button
            className="icon-btn danger"
            disabled={!canDelete}
            onClick={() => canDelete && onSetDirectoryAction && onSetDirectoryAction('delete')}
            style={{
              width: '24px',
              height: '24px',
              opacity: canDelete ? 1 : 0.25,
              cursor: canDelete ? 'pointer' : 'not-allowed',
              background: isDeleteActive ? 'var(--status-danger)' : 'transparent',
              color: isDeleteActive ? '#ffffff' : 'var(--text-secondary)',
            }}
          >
            <Trash2 size={12} />
          </button>
        </Tooltip>

        {/* Skip / Clear Button */}
        <Tooltip content="Skip / Clear folder actions" position="top">
          <button
            className="icon-btn"
            onClick={() => onSetDirectoryAction && onSetDirectoryAction('skip')}
            style={{
              width: '24px',
              height: '24px',
              color: 'var(--text-secondary)',
            }}
          >
            <Minus size={12} />
          </button>
        </Tooltip>

        {/* Folder Diff Button */}
        {hasModifiable && (
          <Tooltip content={tooltipDiff} position="top">
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
  }

  // ─── FILE ACTION BUTTONS ─────────────────────────────────
  if (node.diff_kind.type === 'Identical') {
    return <Check size={12} style={{ color: 'var(--status-success)', opacity: 0.6 }} />;
  }

  // 1. Arrow Right (A ➔ B) Configuration
  const canCopyAtoB = existsInA;
  const isCopyAtoBActive = action === 'copy_a_to_b';
  const tooltipAtoB = !canCopyAtoB
    ? 'File does not exist in Directory A'
    : existsInB
    ? node.diff_kind.type === 'Modified' && node.diff_kind.a_newer
      ? 'Overwrite B (A is newer)'
      : 'Overwrite B'
    : 'Copy to Directory B';

  // 2. Arrow Left (B ⬅ A) Configuration
  const canCopyBtoA = existsInB;
  const isCopyBtoAActive = action === 'copy_b_to_a';
  const tooltipBtoA = !canCopyBtoA
    ? 'File does not exist in Directory B'
    : existsInA
    ? node.diff_kind.type === 'Modified' && node.diff_kind.b_newer
      ? 'Overwrite A (B is newer)'
      : 'Overwrite A'
    : 'Copy to Directory A';

  // 3. Delete Configuration
  const isBoth = existsInA && existsInB;
  const targetDeleteAction: PlannedAction = isBoth
    ? 'delete_both'
    : existsInA
    ? 'delete_a'
    : 'delete_b';
  const isDeleteActive = action === targetDeleteAction;
  const tooltipDelete = isBoth
    ? 'Delete from both A and B'
    : existsInA
    ? 'Delete from Directory A'
    : 'Delete from Directory B';

  const handleDelete = () => {
    onSetAction(isDeleteActive ? 'skip' : targetDeleteAction);
  };

  return (
    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '3px' }}>
      {/* Copy A to B Button */}
      <Tooltip content={tooltipAtoB} position="top">
        <button
          className="icon-btn"
          disabled={!canCopyAtoB}
          onClick={() => canCopyAtoB && onSetAction(isCopyAtoBActive ? 'skip' : 'copy_a_to_b')}
          style={{
            width: '24px',
            height: '24px',
            opacity: canCopyAtoB ? 1 : 0.25,
            cursor: canCopyAtoB ? 'pointer' : 'not-allowed',
            background: isCopyAtoBActive ? 'var(--status-info)' : 'transparent',
            color: isCopyAtoBActive ? '#ffffff' : 'var(--text-secondary)',
          }}
        >
          <ArrowRight size={12} />
        </button>
      </Tooltip>

      {/* Copy B to A Button */}
      <Tooltip content={tooltipBtoA} position="top">
        <button
          className="icon-btn"
          disabled={!canCopyBtoA}
          onClick={() => canCopyBtoA && onSetAction(isCopyBtoAActive ? 'skip' : 'copy_b_to_a')}
          style={{
            width: '24px',
            height: '24px',
            opacity: canCopyBtoA ? 1 : 0.25,
            cursor: canCopyBtoA ? 'pointer' : 'not-allowed',
            background: isCopyBtoAActive ? 'var(--status-info)' : 'transparent',
            color: isCopyBtoAActive ? '#ffffff' : 'var(--text-secondary)',
          }}
        >
          <ArrowLeftIcon size={12} />
        </button>
      </Tooltip>

      {/* Delete Button */}
      <Tooltip content={tooltipDelete} position="top">
        <button
          className="icon-btn danger"
          onClick={handleDelete}
          style={{
            width: '24px',
            height: '24px',
            background: isDeleteActive ? 'var(--status-danger)' : 'transparent',
            color: isDeleteActive ? '#ffffff' : 'var(--text-secondary)',
          }}
        >
          <Trash2 size={12} />
        </button>
      </Tooltip>

      {/* Skip Button */}
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

      {/* Inspect Diff Button for Modified Files */}
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
