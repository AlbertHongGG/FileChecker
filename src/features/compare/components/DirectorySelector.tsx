import React from 'react';
import { Folder, FolderPlus, FolderSync, ExternalLink } from 'lucide-react';
import { Tooltip } from '../../../components/common/Tooltip';
import { TauriClient } from '../../../services/tauriClient';

interface EmptyPaneSlotProps {
  label: string;
  onClick: () => void;
}

export const EmptyPaneSlot: React.FC<EmptyPaneSlotProps> = ({ label, onClick }) => {
  return (
    <div
      onClick={onClick}
      style={{
        flex: 1,
        borderRadius: 'var(--radius-lg)',
        border: '1.5px dashed var(--border-strong)',
        background: 'var(--bg-surface)',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        cursor: 'pointer',
        gap: '8px',
        transition: 'all 0.2s ease',
      }}
      onMouseEnter={(e) => {
        e.currentTarget.style.borderColor = 'var(--status-info)';
        e.currentTarget.style.background = 'var(--bg-surface-hover)';
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.borderColor = 'var(--border-strong)';
        e.currentTarget.style.background = 'var(--bg-surface)';
      }}
    >
      <FolderPlus size={36} strokeWidth={1.5} style={{ color: 'var(--text-secondary)' }} />
      <span style={{ fontSize: '13px', fontWeight: 500, color: 'var(--text-secondary)' }}>
        {label}
      </span>
    </div>
  );
};

interface SelectedPaneHeaderProps {
  label: string;
  dirPath: string;
  onReselect: () => void;
}

export const SelectedPaneHeader: React.FC<SelectedPaneHeaderProps> = ({
  label,
  dirPath,
  onReselect,
}) => {
  const folderName = dirPath.split(/[\\/]/).filter(Boolean).pop() || dirPath;
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '8px 12px',
        background: 'var(--bg-surface)',
        borderRadius: 'var(--radius-sm)',
        border: '1px solid var(--border-subtle)',
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: '8px', minWidth: 0 }}>
        <Folder size={16} style={{ color: 'var(--status-info)', flexShrink: 0 }} />
        <div style={{ minWidth: 0 }}>
          <div style={{ fontSize: '13px', fontWeight: 600, color: 'var(--text-primary)' }}>
            {folderName}
          </div>
          <div
            style={{
              fontSize: '11px',
              color: 'var(--text-secondary)',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
          >
            {dirPath}
          </div>
        </div>
      </div>
      <Tooltip content={`Change ${label}`} position="bottom">
        <button className="icon-btn" onClick={onReselect}>
          <FolderSync size={14} />
        </button>
      </Tooltip>
    </div>
  );
};

interface ActiveHeaderBarProps {
  label: string;
  dirPath: string;
  onReselect: () => void;
}

export const ActiveHeaderBar: React.FC<ActiveHeaderBarProps> = ({
  dirPath,
  onReselect,
}) => {
  const folderName = dirPath.split(/[\\/]/).filter(Boolean).pop() || dirPath;
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '6px 12px',
        minWidth: 0,
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: '6px', minWidth: 0 }}>
        <Folder size={14} style={{ color: 'var(--status-info)', flexShrink: 0 }} />
        <span
          style={{
            fontSize: '12px',
            fontWeight: 600,
            color: 'var(--text-primary)',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}
          title={dirPath}
        >
          {folderName}
        </span>
      </div>
      <div style={{ display: 'flex', alignItems: 'center', gap: '2px', flexShrink: 0 }}>
        <Tooltip content="Open in Explorer" position="bottom">
          <button
            className="icon-btn"
            onClick={() => TauriClient.revealInExplorer(dirPath)}
          >
            <ExternalLink size={12} />
          </button>
        </Tooltip>
        <Tooltip content="Change Directory" position="bottom">
          <button className="icon-btn" onClick={onReselect}>
            <FolderSync size={12} />
          </button>
        </Tooltip>
      </div>
    </div>
  );
};
