import React from 'react';
import { Loader2, Play } from 'lucide-react';
import { Tooltip } from '../../../components/common/Tooltip';
import { SyncProgressPayload } from '../../../types';

interface ExecutionBarProps {
  pendingActionsCount: number;
  isExecuting: boolean;
  syncProgress: SyncProgressPayload | null;
  onExecute: () => void;
}

export const ExecutionBar: React.FC<ExecutionBarProps> = ({
  pendingActionsCount,
  isExecuting,
  syncProgress,
  onExecute,
}) => {
  if (pendingActionsCount === 0) return null;

  return (
    <div
      style={{
        position: 'absolute',
        bottom: '20px',
        right: '24px',
        display: 'flex',
        alignItems: 'center',
        gap: '12px',
        padding: '6px 14px',
        borderRadius: '100px',
        background: 'var(--bg-surface-solid)',
        backdropFilter: 'blur(24px)',
        WebkitBackdropFilter: 'blur(24px)',
        border: '1px solid var(--border-default)',
        boxShadow: 'var(--shadow-lg)',
        zIndex: 80,
      }}
    >
      {isExecuting ? (
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '8px',
            fontSize: '12px',
            color: 'var(--text-secondary)',
          }}
        >
          <Loader2 size={14} className="animate-spin" style={{ color: 'var(--status-info)' }} />
          <span style={{ fontFamily: 'monospace', fontWeight: 600 }}>
            {syncProgress
              ? `${syncProgress.current} / ${syncProgress.total}`
              : `0 / ${pendingActionsCount}`}
          </span>
        </div>
      ) : (
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <span style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-primary)' }}>
            {pendingActionsCount} planned
          </span>
          <Tooltip content="Execute Planned Changes" position="top">
            <button
              className="icon-btn"
              onClick={onExecute}
              style={{
                background: 'var(--status-success)',
                color: '#ffffff',
                width: '28px',
                height: '28px',
              }}
            >
              <Play size={13} fill="#ffffff" />
            </button>
          </Tooltip>
        </div>
      )}
    </div>
  );
};
