import React from 'react';
import { ArrowRight, ArrowLeft, Trash2, FileCode } from 'lucide-react';
import { TreeNode, PlannedAction, DirectoryActionType } from '../../../types';

export interface ActionButtonConfig {
  key: string;
  icon: React.ComponentType<{ size?: number; style?: React.CSSProperties; className?: string }>;
  tooltip: string;
  isActive: boolean;
  isDisabled: boolean;
  isDanger?: boolean;
  activeVariant?: 'info' | 'danger';
  onClick: () => void;
}

/**
 * Resolves exactly 4 action button configurations for a directory row.
 * Invariant: returned array length is strictly 4.
 */
export function resolveDirectoryActionConfigs(
  node: TreeNode,
  activeDirAction: DirectoryActionType | undefined,
  existsInA: boolean,
  existsInB: boolean,
  onSetDirectoryAction?: (actionType: DirectoryActionType) => void,
  onOpenDiff?: () => void
): ActionButtonConfig[] {
  const canMirrorAtoB = existsInA;
  const isMirrorAtoBActive = activeDirAction === 'mirror_a_to_b';
  const tooltipAtoB = !canMirrorAtoB
    ? 'Directory does not exist in Side A'
    : isMirrorAtoBActive
    ? 'Undo Mirror A ➔ B'
    : 'Mirror Subtree A ➔ B (Copy A & Purge extra in B)';

  const canMirrorBtoA = existsInB;
  const isMirrorBtoAActive = activeDirAction === 'mirror_b_to_a';
  const tooltipBtoA = !canMirrorBtoA
    ? 'Directory does not exist in Side B'
    : isMirrorBtoAActive
    ? 'Undo Mirror B ➔ A'
    : 'Mirror Subtree B ➔ A (Copy B & Purge extra in A)';

  const canDelete = existsInA || existsInB;
  const isDeleteActive = activeDirAction === 'delete';
  const tooltipDelete = !canDelete
    ? 'No directory to delete'
    : isDeleteActive
    ? 'Undo Delete Directory'
    : 'Delete Subtree';

  const hasModifiable = Boolean(node.hasModifiable);
  const tooltipDiff = hasModifiable
    ? `Inspect Diff for ${node.modifiableCount || 1} modified file(s)`
    : 'No modified files to inspect in directory';

  return [
    // Slot 1: Mirror A to B
    {
      key: 'mirror_a_to_b',
      icon: ArrowRight,
      tooltip: tooltipAtoB,
      isActive: isMirrorAtoBActive,
      isDisabled: !canMirrorAtoB,
      activeVariant: 'info',
      onClick: () => canMirrorAtoB && onSetDirectoryAction && onSetDirectoryAction('mirror_a_to_b'),
    },
    // Slot 2: Mirror B to A
    {
      key: 'mirror_b_to_a',
      icon: ArrowLeft,
      tooltip: tooltipBtoA,
      isActive: isMirrorBtoAActive,
      isDisabled: !canMirrorBtoA,
      activeVariant: 'info',
      onClick: () => canMirrorBtoA && onSetDirectoryAction && onSetDirectoryAction('mirror_b_to_a'),
    },
    // Slot 3: Delete Subtree
    {
      key: 'delete_subtree',
      icon: Trash2,
      tooltip: tooltipDelete,
      isActive: isDeleteActive,
      isDisabled: !canDelete,
      isDanger: true,
      activeVariant: 'danger',
      onClick: () => canDelete && onSetDirectoryAction && onSetDirectoryAction('delete'),
    },
    // Slot 4: Folder Diff
    {
      key: 'folder_diff',
      icon: FileCode,
      tooltip: tooltipDiff,
      isActive: false,
      isDisabled: !hasModifiable,
      activeVariant: 'info',
      onClick: () => hasModifiable && onOpenDiff && onOpenDiff(),
    },
  ];
}

/**
 * Resolves exactly 4 action button configurations for a file row.
 * Invariant: returned array length is strictly 4.
 */
export function resolveFileActionConfigs(
  node: TreeNode,
  action: PlannedAction,
  existsInA: boolean,
  existsInB: boolean,
  onSetAction: (act: PlannedAction) => void,
  onOpenDiff?: () => void
): ActionButtonConfig[] {
  const canCopyAtoB = existsInA;
  const isCopyAtoBActive = action === 'copy_a_to_b';
  const tooltipAtoB = !canCopyAtoB
    ? 'File does not exist in Side A'
    : existsInB
    ? node.diff_kind.type === 'Modified' && node.diff_kind.a_newer
      ? 'Overwrite Side B (Side A is newer)'
      : 'Overwrite Side B'
    : 'Copy to Side B';

  const canCopyBtoA = existsInB;
  const isCopyBtoAActive = action === 'copy_b_to_a';
  const tooltipBtoA = !canCopyBtoA
    ? 'File does not exist in Side B'
    : existsInA
    ? node.diff_kind.type === 'Modified' && node.diff_kind.b_newer
      ? 'Overwrite Side A (Side B is newer)'
      : 'Overwrite Side A'
    : 'Copy to Side A';

  const isBoth = existsInA && existsInB;
  const canDelete = existsInA || existsInB;
  const targetDeleteAction: PlannedAction = isBoth
    ? 'delete_both'
    : existsInA
    ? 'delete_a'
    : 'delete_b';
  const isDeleteActive = action === targetDeleteAction;
  const tooltipDelete = !canDelete
    ? 'No file to delete'
    : isBoth
    ? 'Delete from both Side A and Side B'
    : existsInA
    ? 'Delete from Side A'
    : 'Delete from Side B';

  const isModified = node.diff_kind.type === 'Modified';
  const tooltipDiff = isModified
    ? 'Inspect File Diff'
    : node.diff_kind.type === 'OnlyInA'
    ? 'File only exists in Side A (No diff to compare)'
    : node.diff_kind.type === 'OnlyInB'
    ? 'File only exists in Side B (No diff to compare)'
    : 'Files are identical';

  return [
    // Slot 1: Copy A to B
    {
      key: 'copy_a_to_b',
      icon: ArrowRight,
      tooltip: tooltipAtoB,
      isActive: isCopyAtoBActive,
      isDisabled: !canCopyAtoB,
      activeVariant: 'info',
      onClick: () => canCopyAtoB && onSetAction(isCopyAtoBActive ? 'skip' : 'copy_a_to_b'),
    },
    // Slot 2: Copy B to A
    {
      key: 'copy_b_to_a',
      icon: ArrowLeft,
      tooltip: tooltipBtoA,
      isActive: isCopyBtoAActive,
      isDisabled: !canCopyBtoA,
      activeVariant: 'info',
      onClick: () => canCopyBtoA && onSetAction(isCopyBtoAActive ? 'skip' : 'copy_b_to_a'),
    },
    // Slot 3: Delete File
    {
      key: 'delete_file',
      icon: Trash2,
      tooltip: tooltipDelete,
      isActive: isDeleteActive,
      isDisabled: !canDelete,
      isDanger: true,
      activeVariant: 'danger',
      onClick: () => canDelete && onSetAction(isDeleteActive ? 'skip' : targetDeleteAction),
    },
    // Slot 4: File Diff
    {
      key: 'file_diff',
      icon: FileCode,
      tooltip: tooltipDiff,
      isActive: false,
      isDisabled: !isModified,
      activeVariant: 'info',
      onClick: () => isModified && onOpenDiff && onOpenDiff(),
    },
  ];
}
