import React from 'react';
import { ActionButtonConfig } from '../../utils/actionButtonResolvers';
import { Tooltip } from '../../../../components/common/Tooltip';

interface ActionIconButtonProps {
  config: ActionButtonConfig;
}

export const ActionIconButton: React.FC<ActionIconButtonProps> = React.memo(({ config }) => {
  const {
    icon: Icon,
    tooltip,
    isActive,
    isDisabled,
    isDanger,
    activeVariant = 'info',
    onClick,
  } = config;

  const activeBg =
    activeVariant === 'danger' ? 'var(--status-danger)' : 'var(--status-info)';

  const activeColor = '#ffffff';

  return (
    <Tooltip content={tooltip} position="top">
      <button
        className={`icon-btn${isDanger ? ' danger' : ''}`}
        disabled={isDisabled}
        onClick={onClick}
        style={{
          width: '24px',
          height: '24px',
          opacity: isDisabled ? 0.25 : 1,
          cursor: isDisabled ? 'not-allowed' : 'pointer',
          background: isActive ? activeBg : 'transparent',
          color: isActive ? activeColor : 'var(--text-secondary)',
          display: 'inline-flex',
          alignItems: 'center',
          justifyContent: 'center',
          padding: 0,
        }}
      >
        <Icon size={12} />
      </button>
    </Tooltip>
  );
});
