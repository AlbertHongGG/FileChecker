import React from 'react';
import {
  Check,
  ArrowRight,
  ArrowLeft as ArrowLeftIcon,
  Trash2,
  Minus,
  FileCode,
} from 'lucide-react';
import { TreeNode, PlannedAction, DirectoryActionType } from '../../../../types';
import { Tooltip } from '../../../../components/common/Tooltip';

interface TreeActionButtonsProps {
  node: TreeNode;
  action: PlannedAction;
  activeDirAction?: DirectoryActionType;
  existsInA: boolean;
  existsInB: boolean;
  onSetAction: (act: PlannedAction) => void;
  onSetDirectoryAction?: (actionType: DirectoryActionType) => void;
  onOpenDiff: () => void;
}

export const TreeActionButtons: React.FC<TreeActionButtonsProps> = ({
  node,
  action,
  activeDirAction,
  existsInA,
  existsInB,
  onSetAction,
  onSetDirectoryAction,
  onOpenDiff,
}) => {
  // ─── DIRECTORY ACTION BUTTONS ────────────────────────────
  if (node.is_dir) {
    if (node.diff_kind.type === 'Identical') {
      return <Check size={12} style={{ color: 'var(--status-success)', opacity: 0.6 }} />;
    }

    // Directory Capabilities: Mirroring requires existing on that side
    const canMirrorAtoB = existsInA;
    const canMirrorBtoA = existsInB;
    const canDelete = existsInA || existsInB;
    const hasModifiable = Boolean(node.hasModifiable);

    // Subtree Policy Active States (O(1) precomputed lookup)
    const isMirrorAtoBActive = activeDirAction === 'mirror_a_to_b';
    const isMirrorBtoAActive = activeDirAction === 'mirror_b_to_a';
    const isDeleteActive = activeDirAction === 'delete';

    const tooltipAtoB = !canMirrorAtoB
      ? 'Directory does not exist in A'
      : isMirrorAtoBActive
      ? 'Undo Mirror A ➔ B'
      : 'Mirror Subtree A ➔ B (Copy A & Purge extra in B)';

    const tooltipBtoA = !canMirrorBtoA
      ? 'Directory does not exist in B'
      : isMirrorBtoAActive
      ? 'Undo Mirror B ➔ A'
      : 'Mirror Subtree B ➔ A (Copy B & Purge extra in A)';

    const tooltipDelete = !canDelete
      ? 'No directory to delete'
      : isDeleteActive
      ? 'Undo Delete Directory'
      : 'Delete Subtree';

    const tooltipDiff = hasModifiable
      ? `Inspect Diff for ${node.modifiableCount || 1} modified file(s)`
      : 'No modified files to inspect in folder';

    return (
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '3px' }}>
        {/* Mirror A to B Button */}
        <Tooltip content={tooltipAtoB} position="top">
          <button
            className="icon-btn"
            disabled={!canMirrorAtoB}
            onClick={() => canMirrorAtoB && onSetDirectoryAction && onSetDirectoryAction('mirror_a_to_b')}
            style={{
              width: '24px',
              height: '24px',
              opacity: canMirrorAtoB ? 1 : 0.25,
              cursor: canMirrorAtoB ? 'pointer' : 'not-allowed',
              background: isMirrorAtoBActive ? 'var(--status-info)' : 'transparent',
              color: isMirrorAtoBActive ? '#ffffff' : 'var(--text-secondary)',
            }}
          >
            <ArrowRight size={12} />
          </button>
        </Tooltip>

        {/* Mirror B to A Button */}
        <Tooltip content={tooltipBtoA} position="top">
          <button
            className="icon-btn"
            disabled={!canMirrorBtoA}
            onClick={() => canMirrorBtoA && onSetDirectoryAction && onSetDirectoryAction('mirror_b_to_a')}
            style={{
              width: '24px',
              height: '24px',
              opacity: canMirrorBtoA ? 1 : 0.25,
              cursor: canMirrorBtoA ? 'pointer' : 'not-allowed',
              background: isMirrorBtoAActive ? 'var(--status-info)' : 'transparent',
              color: isMirrorBtoAActive ? '#ffffff' : 'var(--text-secondary)',
            }}
          >
            <ArrowLeftIcon size={12} />
          </button>
        </Tooltip>

        {/* Delete Subtree Button */}
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
