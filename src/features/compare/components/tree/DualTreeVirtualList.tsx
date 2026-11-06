import React, { useMemo } from 'react';
import { Loader2, X } from 'lucide-react';
import { FlattenedRow, PlannedAction, ScanProgressPayload } from '../../../../types';
import { DualTreeRow } from './DualTreeRow';

interface DualTreeVirtualListProps {
  scrollContainerRef: React.RefObject<HTMLDivElement | null>;
  onScroll: (e: React.UIEvent<HTMLDivElement>) => void;
  isLoading: boolean;
  scanProgress: ScanProgressPayload | null;
  onlyDiffs: boolean;
  rowsCount: number;
  visibleRows: FlattenedRow[];
  totalHeight: number;
  paddingTop: number;
  actionMap: Map<string, PlannedAction>;
  onToggleExpand: (path: string) => void;
  onSetAction: (path: string, act: PlannedAction) => void;
  onOpenDiff: (path: string) => void;
  onCancelScan?: () => void;
}

export const DualTreeVirtualList: React.FC<DualTreeVirtualListProps> = ({
  scrollContainerRef,
  onScroll,
  isLoading,
  scanProgress,
  onlyDiffs,
  rowsCount,
  visibleRows,
  totalHeight,
  paddingTop,
  actionMap,
  onToggleExpand,
  onSetAction,
  onOpenDiff,
  onCancelScan,
}) => {
  const percentA = useMemo(() => {
    if (!scanProgress || scanProgress.total_a === 0) return 0;
    return Math.min(100, Math.round((scanProgress.completed_a / scanProgress.total_a) * 100));
  }, [scanProgress]);

  const percentB = useMemo(() => {
    if (!scanProgress || scanProgress.total_b === 0) return 0;
    return Math.min(100, Math.round((scanProgress.completed_b / scanProgress.total_b) * 100));
  }, [scanProgress]);

  return (
    <div
      ref={scrollContainerRef}
      onScroll={onScroll}
      style={{
        flex: 1,
        overflowY: 'auto',
        overflowX: 'hidden',
        position: 'relative',
      }}
    >
      {isLoading && rowsCount === 0 ? (
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            height: '100%',
            gap: '16px',
            color: 'var(--text-secondary)',
            padding: '24px',
          }}
        >
          <div
            style={{
              display: 'flex',
              flexDirection: 'column',
              width: '100%',
              maxWidth: '440px',
              padding: '20px 24px',
              borderRadius: '12px',
              background: 'var(--bg-surface)',
              border: '1px solid var(--border-subtle)',
              boxShadow: '0 8px 32px rgba(0,0,0,0.12)',
              gap: '16px',
            }}
          >
            {/* Header with spinner and cancel */}
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <Loader2 size={16} className="animate-spin" style={{ color: 'var(--status-info)' }} />
                <span style={{ fontSize: '13px', fontWeight: 600, color: 'var(--text-primary)' }}>
                  Scanning Directories...
                </span>
              </div>
              {onCancelScan && (
                <button
                  className="icon-btn"
                  onClick={onCancelScan}
                  title="Cancel Scan"
                  style={{ color: 'var(--status-danger)' }}
                >
                  <X size={15} />
                </button>
              )}
            </div>

            {/* Side A Anchor Progress */}
            <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '12px' }}>
                <span style={{ fontWeight: 600, color: 'var(--text-primary)' }}>
                  Directory A ({scanProgress?.is_hdd_a ? 'HDD' : 'SSD'})
                </span>
                <span style={{ fontFamily: 'monospace', fontWeight: 600, color: 'var(--status-info)' }}>
                  {percentA}%
                </span>
              </div>
              <div
                style={{
                  width: '100%',
                  height: '6px',
                  borderRadius: '3px',
                  background: 'var(--bg-surface-hover)',
                  overflow: 'hidden',
                }}
              >
                <div
                  style={{
                    width: `${percentA}%`,
                    height: '100%',
                    background: 'var(--status-info)',
                    borderRadius: '3px',
                    transition: 'width 0.2s ease',
                  }}
                />
              </div>
              <div
                style={{
                  display: 'flex',
                  justifyContent: 'space-between',
                  fontSize: '11px',
                  color: 'var(--text-tertiary)',
                }}
              >
                <span
                  style={{
                    maxWidth: '240px',
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                  }}
                >
                  {scanProgress?.current_name_a || 'Starting...'}
                </span>
                <span style={{ fontFamily: 'monospace' }}>
                  {scanProgress?.completed_a}/{scanProgress?.total_a} ({scanProgress?.count_a.toLocaleString()} files)
                </span>
              </div>
            </div>

            {/* Side B Anchor Progress */}
            <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '12px' }}>
                <span style={{ fontWeight: 600, color: 'var(--text-primary)' }}>
                  Directory B ({scanProgress?.is_hdd_b ? 'HDD' : 'SSD'})
                </span>
                <span style={{ fontFamily: 'monospace', fontWeight: 600, color: 'var(--status-info)' }}>
                  {percentB}%
                </span>
              </div>
              <div
                style={{
                  width: '100%',
                  height: '6px',
                  borderRadius: '3px',
                  background: 'var(--bg-surface-hover)',
                  overflow: 'hidden',
                }}
              >
                <div
                  style={{
                    width: `${percentB}%`,
                    height: '100%',
                    background: 'var(--status-info)',
                    borderRadius: '3px',
                    transition: 'width 0.2s ease',
                  }}
                />
              </div>
              <div
                style={{
                  display: 'flex',
                  justifyContent: 'space-between',
                  fontSize: '11px',
                  color: 'var(--text-tertiary)',
                }}
              >
                <span
                  style={{
                    maxWidth: '240px',
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                  }}
                >
                  {scanProgress?.current_name_b || 'Starting...'}
                </span>
                <span style={{ fontFamily: 'monospace' }}>
                  {scanProgress?.completed_b}/{scanProgress?.total_b} ({scanProgress?.count_b.toLocaleString()} files)
                </span>
              </div>
            </div>
          </div>
        </div>
      ) : rowsCount === 0 ? (
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            height: '100%',
            color: 'var(--text-tertiary)',
            fontSize: '13px',
          }}
        >
          {onlyDiffs ? 'Identical (0 differences)' : 'Empty directory'}
        </div>
      ) : (
        <div style={{ height: `${totalHeight}px`, position: 'relative' }}>
          <div style={{ transform: `translateY(${paddingTop}px)` }}>
            {visibleRows.map((row) => (
              <DualTreeRow
                key={row.node.relative_path}
                row={row}
                action={actionMap.get(row.node.relative_path) || 'skip'}
                onToggleExpand={() => onToggleExpand(row.node.relative_path)}
                onSetAction={(act) => onSetAction(row.node.relative_path, act)}
                onOpenDiff={() => onOpenDiff(row.node.relative_path)}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  );
};
