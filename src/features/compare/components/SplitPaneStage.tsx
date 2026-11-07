import React from 'react';
import {
  FolderPlus,
  FolderCheck,
  FolderSync,
  Loader2,
  HardDrive,
  Cpu,
  X,
} from 'lucide-react';
import { ScanProgressPayload } from '../../../types';

export interface SideScanMetrics {
  completed: number;
  total: number;
  currentName: string;
  count: number;
  isHdd: boolean;
}

interface PaneStatusCardProps {
  side: 'A' | 'B';
  dirPath: string | null;
  isLoading: boolean;
  metrics: SideScanMetrics | null;
  onSelect: () => void;
}

export const PaneStatusCard: React.FC<PaneStatusCardProps> = ({
  side,
  dirPath,
  isLoading,
  metrics,
  onSelect,
}) => {
  const label = `Directory ${side}`;

  // State 1: Empty - prompt user to select directory
  if (!dirPath) {
    return (
      <div
        onClick={onSelect}
        style={{
          flex: 1,
          margin: '16px',
          borderRadius: '12px',
          border: '1.5px dashed var(--border-strong)',
          background: 'var(--bg-surface)',
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          cursor: 'pointer',
          gap: '10px',
          transition: 'all 0.2s ease',
          userSelect: 'none',
        }}
        onMouseEnter={(e) => {
          e.currentTarget.style.borderColor = 'var(--status-info)';
          e.currentTarget.style.backgroundColor = 'var(--bg-surface-hover)';
        }}
        onMouseLeave={(e) => {
          e.currentTarget.style.borderColor = 'var(--border-strong)';
          e.currentTarget.style.backgroundColor = 'var(--bg-surface)';
        }}
      >
        <div
          style={{
            width: '48px',
            height: '48px',
            borderRadius: '50%',
            background: 'var(--bg-base)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            color: 'var(--text-secondary)',
          }}
        >
          <FolderPlus size={24} />
        </div>
        <div style={{ textAlign: 'center' }}>
          <div style={{ fontSize: '13px', fontWeight: 600, color: 'var(--text-primary)' }}>
            Select {label}
          </div>
          <div style={{ fontSize: '11px', color: 'var(--text-tertiary)', marginTop: '2px' }}>
            Click to choose a folder to compare
          </div>
        </div>
      </div>
    );
  }

  // State 3: Scanning in progress
  if (isLoading) {
    const percent =
      metrics && metrics.total > 0
        ? Math.min(100, Math.round((metrics.completed / metrics.total) * 100))
        : 0;

    return (
      <div
        style={{
          flex: 1,
          margin: '16px',
          borderRadius: '12px',
          border: '1px solid var(--border-subtle)',
          background: 'var(--bg-surface)',
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          padding: '24px',
          boxSizing: 'border-box',
          gap: '16px',
        }}
      >
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            width: '100%',
            maxWidth: '320px',
            gap: '14px',
          }}
        >
          {/* Header with media badge & percentage */}
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
              <span
                style={{
                  fontSize: '11px',
                  fontWeight: 600,
                  padding: '2px 6px',
                  borderRadius: '4px',
                  background: 'var(--status-info-light)',
                  color: 'var(--status-info)',
                  display: 'flex',
                  alignItems: 'center',
                  gap: '4px',
                }}
              >
                {metrics?.isHdd ? <HardDrive size={11} /> : <Cpu size={11} />}
                {metrics?.isHdd ? 'HDD (Sequential)' : 'SSD (Parallel)'}
              </span>
            </div>
            <span style={{ fontSize: '13px', fontWeight: 700, color: 'var(--text-primary)', fontFamily: 'monospace' }}>
              {percent}%
            </span>
          </div>

          {/* Progress Bar */}
          <div
            style={{
              width: '100%',
              height: '6px',
              borderRadius: '3px',
              background: 'var(--border-subtle)',
              overflow: 'hidden',
            }}
          >
            <div
              style={{
                width: `${percent}%`,
                height: '100%',
                background: 'var(--status-info)',
                transition: 'width 0.15s ease',
                borderRadius: '3px',
              }}
            />
          </div>

          {/* Current Anchor & Count Stats */}
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              fontSize: '11px',
              color: 'var(--text-tertiary)',
            }}
          >
            <span
              style={{
                maxWidth: '180px',
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
                color: 'var(--text-secondary)',
              }}
              title={metrics?.currentName || 'Preparing...'}
            >
              {metrics?.currentName ? `[${metrics.currentName}]` : 'Scanning...'}
            </span>
            <span style={{ fontFamily: 'monospace' }}>
              {metrics?.completed || 0}/{metrics?.total || 0} ({metrics?.count.toLocaleString() || 0} files)
            </span>
          </div>
        </div>
      </div>
    );
  }

  // State 2: Mounted (Selected & Waiting for counterpart)
  const folderName = dirPath.split(/[\\/]/).filter(Boolean).pop() || dirPath;

  return (
    <div
      style={{
        flex: 1,
        margin: '16px',
        borderRadius: '12px',
        border: '1px solid var(--border-subtle)',
        background: 'var(--bg-surface)',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        padding: '24px',
        gap: '12px',
      }}
    >
      <div
        style={{
          width: '48px',
          height: '48px',
          borderRadius: '50%',
          background: 'var(--status-success-light)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          color: 'var(--status-success)',
        }}
      >
        <FolderCheck size={24} />
      </div>

      <div style={{ textAlign: 'center', maxWidth: '360px' }}>
        <div style={{ fontSize: '14px', fontWeight: 600, color: 'var(--text-primary)' }}>
          {folderName}
        </div>
        <div
          style={{
            fontSize: '11px',
            color: 'var(--text-tertiary)',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            marginTop: '2px',
          }}
          title={dirPath}
        >
          {dirPath}
        </div>
      </div>

      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '6px',
          fontSize: '11px',
          color: 'var(--status-success)',
          marginTop: '4px',
        }}
      >
        <span style={{ width: '6px', height: '6px', borderRadius: '50%', background: 'var(--status-success)' }} />
        <span>Mounted & Ready</span>
      </div>

      <button
        onClick={onSelect}
        className="icon-btn"
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '6px',
          padding: '4px 10px',
          borderRadius: '6px',
          border: '1px solid var(--border-subtle)',
          fontSize: '11px',
          color: 'var(--text-secondary)',
          marginTop: '6px',
        }}
      >
        <FolderSync size={12} />
        <span>Change</span>
      </button>
    </div>
  );
};

interface SplitPaneStageProps {
  dirA: string | null;
  dirB: string | null;
  isLoading: boolean;
  scanProgress: ScanProgressPayload | null;
  onSelectDirA: () => void;
  onSelectDirB: () => void;
  onCancelScan?: () => void;
}

export const SplitPaneStage: React.FC<SplitPaneStageProps> = ({
  dirA,
  dirB,
  isLoading,
  scanProgress,
  onSelectDirA,
  onSelectDirB,
  onCancelScan,
}) => {
  const metricsA: SideScanMetrics | null = scanProgress
    ? {
        completed: scanProgress.completed_a,
        total: scanProgress.total_a,
        currentName: scanProgress.current_name_a,
        count: scanProgress.count_a,
        isHdd: scanProgress.is_hdd_a,
      }
    : null;

  const metricsB: SideScanMetrics | null = scanProgress
    ? {
        completed: scanProgress.completed_b,
        total: scanProgress.total_b,
        currentName: scanProgress.current_name_b,
        count: scanProgress.count_b,
        isHdd: scanProgress.is_hdd_b,
      }
    : null;

  return (
    <div
      style={{
        display: 'grid',
        gridTemplateColumns: '1fr 140px 1fr',
        flex: 1,
        overflow: 'hidden',
        background: 'var(--bg-base)',
      }}
    >
      {/* Left Column: Side A Status */}
      <div style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
        <PaneStatusCard
          side="A"
          dirPath={dirA}
          isLoading={isLoading}
          metrics={metricsA}
          onSelect={onSelectDirA}
        />
      </div>

      {/* Middle Column: Central Action Axis */}
      <div
        style={{
          borderLeft: '1px solid var(--border-subtle)',
          borderRight: '1px solid var(--border-subtle)',
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          gap: '12px',
          userSelect: 'none',
        }}
      >
        {isLoading ? (
          <>
            <Loader2
              size={20}
              className="animate-spin"
              style={{ color: 'var(--status-info)' }}
            />
            {onCancelScan && (
              <button
                className="icon-btn"
                onClick={onCancelScan}
                title="Cancel Scan"
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '4px',
                  color: 'var(--status-danger)',
                  fontSize: '11px',
                  padding: '3px 8px',
                  borderRadius: '4px',
                  border: '1px solid var(--status-danger-light)',
                  background: 'var(--bg-surface)',
                }}
              >
                <X size={12} />
                <span>Cancel</span>
              </button>
            )}
          </>
        ) : (
          <div
            style={{
              width: '1px',
              height: '40px',
              background: 'var(--border-subtle)',
            }}
          />
        )}
      </div>

      {/* Right Column: Side B Status */}
      <div style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
        <PaneStatusCard
          side="B"
          dirPath={dirB}
          isLoading={isLoading}
          metrics={metricsB}
          onSelect={onSelectDirB}
        />
      </div>
    </div>
  );
};
