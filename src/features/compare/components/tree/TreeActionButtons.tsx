import React from 'react';
import { Check } from 'lucide-react';
import { TreeNode, PlannedAction, DirectoryActionType } from '../../../../types';
import {
  resolveDirectoryActionConfigs,
  resolveFileActionConfigs,
} from '../../utils/actionButtonResolvers';
import { ActionIconButton } from './ActionIconButton';

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

export const TreeActionButtons: React.FC<TreeActionButtonsProps> = React.memo(({
  node,
  action,
  activeDirAction,
  existsInA,
  existsInB,
  onSetAction,
  onSetDirectoryAction,
  onOpenDiff,
}) => {
  if (node.diff_kind.type === 'Identical') {
    return (
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', width: '100%' }}>
        <Check size={12} style={{ color: 'var(--status-success)', opacity: 0.6 }} />
      </div>
    );
  }

  const buttonConfigs = node.is_dir
    ? resolveDirectoryActionConfigs(
        node,
        activeDirAction,
        existsInA,
        existsInB,
        onSetDirectoryAction,
        onOpenDiff
      )
    : resolveFileActionConfigs(
        node,
        action,
        existsInA,
        existsInB,
        onSetAction,
        onOpenDiff
      );

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        gap: '3px',
        width: '100%',
      }}
    >
      {buttonConfigs.map((config) => (
        <ActionIconButton key={config.key} config={config} />
      ))}
    </div>
  );
});
