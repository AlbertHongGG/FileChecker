import React from 'react';
import { Loader2 } from 'lucide-react';
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
}) => {
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
            gap: '10px',
            color: 'var(--text-secondary)',
          }}
        >
          <Loader2 size={24} className="animate-spin" style={{ color: 'var(--status-info)' }} />
          {scanProgress && (
            <span style={{ fontSize: '12px' }}>
              A: {scanProgress.count_a.toLocaleString()} {scanProgress.is_hdd_a ? '(HDD)' : '(SSD)'} &nbsp;|&nbsp; B:{' '}
              {scanProgress.count_b.toLocaleString()} {scanProgress.is_hdd_b ? '(HDD)' : '(SSD)'}
            </span>
          )}
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
