import React from 'react';
import {
  Shield,
  ShieldAlert,
  Filter,
  Sparkles,
  ArrowRightToLine,
  ArrowLeftToLine,
  RotateCcw,
} from 'lucide-react';
import { Tooltip } from '../../../components/common/Tooltip';

interface PresetActionsProps {
  pruneIgnored: boolean;
  onTogglePrune: () => void;
  entriesCount: number;
  onlyDiffs: boolean;
  onToggleOnlyDiffs: () => void;
  onApplyPresetNewer: () => void;
  onApplyPresetMirrorAtoB: () => void;
  onApplyPresetMirrorBtoA: () => void;
  onResetActions: () => void;
  isExecuting: boolean;
  isLoading: boolean;
  pendingActionsCount: number;
}

export const PresetActions: React.FC<PresetActionsProps> = ({
  pruneIgnored,
  onTogglePrune,
  entriesCount,
  onlyDiffs,
  onToggleOnlyDiffs,
  onApplyPresetNewer,
  onApplyPresetMirrorAtoB,
  onApplyPresetMirrorBtoA,
  onResetActions,
  isExecuting,
  isLoading,
  pendingActionsCount,
}) => {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
      {/* Smart Pruning Toggle */}
      <Tooltip
        content={
          pruneIgnored
            ? 'Smart Prune: ON (Skips node_modules/target/git)'
            : 'Smart Prune: OFF (Scan all files)'
        }
        position="bottom"
      >
        <button
          className="icon-btn"
          onClick={onTogglePrune}
          style={{ color: pruneIgnored ? 'var(--status-success)' : 'var(--status-warning)' }}
        >
          {pruneIgnored ? <Shield size={14} /> : <ShieldAlert size={14} />}
        </button>
      </Tooltip>

      {entriesCount > 0 && (
        <>
          <Tooltip content={onlyDiffs ? 'Show All Items' : 'Show Only Differences'} position="bottom">
            <button
              className="icon-btn"
              onClick={onToggleOnlyDiffs}
              style={{ color: onlyDiffs ? 'var(--status-info)' : 'var(--text-secondary)' }}
            >
              <Filter size={14} />
            </button>
          </Tooltip>

          <div
            style={{
              width: '1px',
              height: '18px',
              background: 'var(--border-default)',
              margin: '0 4px',
            }}
          />

          <Tooltip content="Sync Newer Files" position="bottom">
            <button
              className="icon-btn"
              onClick={onApplyPresetNewer}
              disabled={isExecuting || isLoading}
            >
              <Sparkles size={14} />
            </button>
          </Tooltip>

          <Tooltip content="Mirror A to B" position="bottom">
            <button
              className="icon-btn"
              onClick={onApplyPresetMirrorAtoB}
              disabled={isExecuting || isLoading}
            >
              <ArrowRightToLine size={14} />
            </button>
          </Tooltip>

          <Tooltip content="Mirror B to A" position="bottom">
            <button
              className="icon-btn"
              onClick={onApplyPresetMirrorBtoA}
              disabled={isExecuting || isLoading}
            >
              <ArrowLeftToLine size={14} />
            </button>
          </Tooltip>

          <Tooltip content="Reset All Actions" position="bottom">
            <button
              className="icon-btn"
              onClick={onResetActions}
              disabled={isExecuting || isLoading || pendingActionsCount === 0}
            >
              <RotateCcw size={14} />
            </button>
          </Tooltip>
        </>
      )}
    </div>
  );
};
