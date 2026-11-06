import React from 'react';
import { ArrowLeft, RefreshCw, X, Loader2 } from 'lucide-react';
import { Tooltip } from '../../../components/common/Tooltip';
import { ScanProgressPayload } from '../../../types';
import { PresetActions } from './PresetActions';

interface CompareToolbarProps {
  onBack: () => void;
  dirA: string | null;
  dirB: string | null;
  isLoading: boolean;
  isExecuting: boolean;
  scanProgress: ScanProgressPayload | null;
  entriesCount: number;
  stats: {
    diffCount: number;
    onlyACount: number;
    onlyBCount: number;
    modifiedCount: number;
  };
  pruneIgnored: boolean;
  onTogglePrune: () => void;
  onlyDiffs: boolean;
  onToggleOnlyDiffs: () => void;
  onApplyPresetNewer: () => void;
  onApplyPresetMirrorAtoB: () => void;
  onApplyPresetMirrorBtoA: () => void;
  onResetActions: () => void;
  pendingActionsCount: number;
  onRescan: () => void;
  onCancelScan: () => void;
}

export const CompareToolbar: React.FC<CompareToolbarProps> = ({
  onBack,
  dirA,
  dirB,
  isLoading,
  isExecuting,
  scanProgress,
  entriesCount,
  stats,
  pruneIgnored,
  onTogglePrune,
  onlyDiffs,
  onToggleOnlyDiffs,
  onApplyPresetNewer,
  onApplyPresetMirrorAtoB,
  onApplyPresetMirrorBtoA,
  onResetActions,
  pendingActionsCount,
  onRescan,
  onCancelScan,
}) => {
  return (
    <div
      style={{
        height: '44px',
        padding: '0 12px',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        borderBottom: '1px solid var(--border-subtle)',
        background: 'var(--bg-surface)',
        gap: '8px',
      }}
    >
      {/* Left: Navigation, Status & Summary */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '6px', minWidth: 0 }}>
        <Tooltip content="Back to Portal" position="bottom">
          <button className="icon-btn" onClick={onBack}>
            <ArrowLeft size={15} />
          </button>
        </Tooltip>

        {dirA && dirB && (
          <Tooltip content={isLoading ? 'Cancel Scan' : 'Rescan'} position="bottom">
            <button
              className="icon-btn"
              onClick={isLoading ? onCancelScan : onRescan}
              disabled={isExecuting}
              style={{ color: isLoading ? 'var(--status-danger)' : 'inherit' }}
            >
              {isLoading ? <X size={14} /> : <RefreshCw size={14} />}
            </button>
          </Tooltip>
        )}

        {/* Live dual-side scanning indicator */}
        {isLoading && scanProgress && (
          <div style={{ display: 'flex', alignItems: 'center', gap: '6px', marginLeft: '6px' }}>
            <Loader2 size={13} className="animate-spin" style={{ color: 'var(--status-info)' }} />
            <span style={{ fontSize: '11px', color: 'var(--text-secondary)', whiteSpace: 'nowrap' }}>
              A: {scanProgress.count_a.toLocaleString()} {scanProgress.is_hdd_a ? '(HDD)' : '(SSD)'} &nbsp;|&nbsp; B:{' '}
              {scanProgress.count_b.toLocaleString()} {scanProgress.is_hdd_b ? '(HDD)' : '(SSD)'}
            </span>
          </div>
        )}

        {/* Summary Badges */}
        {!isLoading && entriesCount > 0 && (
          <div style={{ display: 'flex', alignItems: 'center', gap: '6px', marginLeft: '6px' }}>
            <span
              style={{
                fontSize: '11px',
                fontWeight: 600,
                padding: '2px 8px',
                borderRadius: '100px',
                background:
                  stats.diffCount > 0
                    ? 'var(--status-warning-light)'
                    : 'var(--status-success-light)',
                color:
                  stats.diffCount > 0 ? 'var(--status-warning)' : 'var(--status-success)',
              }}
            >
              {stats.diffCount} diffs
            </span>
            <span
              style={{
                fontSize: '11px',
                padding: '2px 6px',
                borderRadius: '4px',
                background: 'var(--bg-surface-hover)',
                color: 'var(--text-secondary)',
              }}
            >
              +{stats.onlyACount} A
            </span>
            <span
              style={{
                fontSize: '11px',
                padding: '2px 6px',
                borderRadius: '4px',
                background: 'var(--bg-surface-hover)',
                color: 'var(--text-secondary)',
              }}
            >
              +{stats.onlyBCount} B
            </span>
            <span
              style={{
                fontSize: '11px',
                padding: '2px 6px',
                borderRadius: '4px',
                background: 'var(--bg-surface-hover)',
                color: 'var(--text-secondary)',
              }}
            >
              ~{stats.modifiedCount} mod
            </span>
          </div>
        )}
      </div>

      {/* Right: Prune Toggle, Filter & Batch Presets */}
      <PresetActions
        pruneIgnored={pruneIgnored}
        onTogglePrune={onTogglePrune}
        entriesCount={entriesCount}
        onlyDiffs={onlyDiffs}
        onToggleOnlyDiffs={onToggleOnlyDiffs}
        onApplyPresetNewer={onApplyPresetNewer}
        onApplyPresetMirrorAtoB={onApplyPresetMirrorAtoB}
        onApplyPresetMirrorBtoA={onApplyPresetMirrorBtoA}
        onResetActions={onResetActions}
        isExecuting={isExecuting}
        isLoading={isLoading}
        pendingActionsCount={pendingActionsCount}
      />
    </div>
  );
};
