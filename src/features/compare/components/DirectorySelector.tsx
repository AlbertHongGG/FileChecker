import React from 'react';
import { Folder, FolderPlus, FolderSync, ExternalLink } from 'lucide-react';
import { Tooltip } from '../../../components/common/Tooltip';
import { TauriClient } from '../../../services/tauriClient';

interface PaneHeaderCellProps {
  side: 'A' | 'B';
  dirPath: string | null;
  isDisabled: boolean;
  onSelect: () => void;
}

export const PaneHeaderCell: React.FC<PaneHeaderCellProps> = ({
  side,
  dirPath,
  isDisabled,
  onSelect,
}) => {
  const label = `Directory ${side}`;

  if (!dirPath) {
    return (
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '6px 12px',
          height: '36px',
          boxSizing: 'border-box',
        }}
      >
        <button
          onClick={onSelect}
          disabled={isDisabled}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '6px',
            background: 'transparent',
            border: 'none',
            color: 'var(--text-tertiary)',
            cursor: isDisabled ? 'not-allowed' : 'pointer',
            fontSize: '12px',
            padding: 0,
            transition: 'color 0.15s ease',
          }}
          onMouseEnter={(e) => {
            if (!isDisabled) e.currentTarget.style.color = 'var(--status-info)';
          }}
          onMouseLeave={(e) => {
            if (!isDisabled) e.currentTarget.style.color = 'var(--text-tertiary)';
          }}
        >
          <FolderPlus size={14} style={{ opacity: 0.8 }} />
          <span>Select {label}...</span>
        </button>
      </div>
    );
  }

  const folderName = dirPath.split(/[\\/]/).filter(Boolean).pop() || dirPath;

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '6px 12px',
        height: '36px',
        boxSizing: 'border-box',
        minWidth: 0,
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: '6px', minWidth: 0 }}>
        <Folder size={14} style={{ color: 'var(--status-info)', flexShrink: 0 }} />
        <Tooltip content={dirPath} position="bottom">
          <span
            style={{
              fontSize: '12px',
              fontWeight: 600,
              color: 'var(--text-primary)',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
              cursor: 'default',
            }}
          >
            {folderName}
          </span>
        </Tooltip>
      </div>

      <div style={{ display: 'flex', alignItems: 'center', gap: '2px', flexShrink: 0 }}>
        <Tooltip content="Open in Explorer" position="bottom">
          <button
            className="icon-btn"
            disabled={isDisabled}
            onClick={() => TauriClient.revealInExplorer(dirPath)}
          >
            <ExternalLink size={12} />
          </button>
        </Tooltip>
        <Tooltip content={`Change ${label}`} position="bottom">
          <button className="icon-btn" disabled={isDisabled} onClick={onSelect}>
            <FolderSync size={12} />
          </button>
        </Tooltip>
      </div>
    </div>
  );
};

interface WorkspaceHeaderRowProps {
  dirA: string | null;
  dirB: string | null;
  showActionColumn: boolean;
  isLoading: boolean;
  isExecuting: boolean;
  onSelectDirA: () => void;
  onSelectDirB: () => void;
}

export const WorkspaceHeaderRow: React.FC<WorkspaceHeaderRowProps> = ({
  dirA,
  dirB,
  showActionColumn,
  isLoading,
  isExecuting,
  onSelectDirA,
  onSelectDirB,
}) => {
  const isActionDisabled = isLoading || isExecuting;

  return (
    <div
      style={{
        display: 'grid',
        gridTemplateColumns: showActionColumn ? '1fr 140px 1fr' : '1fr 1fr',
        borderBottom: '1px solid var(--border-subtle)',
        background: 'var(--bg-surface)',
        alignItems: 'center',
        flexShrink: 0,
        height: '36px',
        boxSizing: 'border-box',
      }}
    >
      {/* Directory A Header Cell */}
      <div
        style={{
          borderRight: !showActionColumn ? '1px solid var(--border-subtle)' : 'none',
          height: '100%',
          display: 'flex',
          alignItems: 'center',
        }}
      >
        <div style={{ width: '100%' }}>
          <PaneHeaderCell
            side="A"
            dirPath={dirA}
            isDisabled={isActionDisabled}
            onSelect={onSelectDirA}
          />
        </div>
      </div>

      {/* Center Action Column Header (Only rendered when showActionColumn is true) */}
      {showActionColumn && (
        <div
          style={{
            textAlign: 'center',
            fontSize: '11px',
            fontWeight: 600,
            color: 'var(--text-tertiary)',
            borderLeft: '1px solid var(--border-subtle)',
            borderRight: '1px solid var(--border-subtle)',
            height: '100%',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            userSelect: 'none',
          }}
        >
          Action
        </div>
      )}

      {/* Directory B Header Cell */}
      <div style={{ height: '100%', display: 'flex', alignItems: 'center' }}>
        <div style={{ width: '100%' }}>
          <PaneHeaderCell
            side="B"
            dirPath={dirB}
            isDisabled={isActionDisabled}
            onSelect={onSelectDirB}
          />
        </div>
      </div>
    </div>
  );
};
