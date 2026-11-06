import { useState, useEffect, useMemo, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { listen } from '@tauri-apps/api/event';
import {
  ArrowLeft,
  RefreshCw,
  Filter,
  Sparkles,
  ArrowRightToLine,
  ArrowLeftToLine,
  RotateCcw,
  FolderPlus,
  FolderSync,
  ExternalLink,
  ChevronRight,
  ChevronDown,
  FileCode,
  Folder,
  ArrowRight,
  ArrowLeft as ArrowLeftIcon,
  Trash2,
  Minus,
  Check,
  FileText,
  Play,
  Loader2,
  Shield,
  ShieldAlert,
  X,
} from 'lucide-react';
import { Tooltip } from './Tooltip';
import { useNotification } from '../contexts/NotificationContext';
import { DiffDrawer } from './DiffDrawer';
import {
  UnifiedEntry,
  PlannedAction,
  SyncTask,
  ScanProgressPayload,
  SyncProgressPayload,
} from '../types/compare';
import {
  buildTree,
  flattenTree,
  formatFileSize,
  formatTimestamp,
  FlattenedRow,
} from '../utils/treeUtils';

const ROW_HEIGHT = 28;
const BUFFER_SIZE = 12;

interface CompareWorkspaceProps {
  onBack: () => void;
}

export default function CompareWorkspace({ onBack }: CompareWorkspaceProps) {
  const [dirA, setDirA] = useState<string | null>(null);
  const [dirB, setDirB] = useState<string | null>(null);

  const [isLoading, setIsLoading] = useState(false);
  const [scanProgress, setScanProgress] = useState<ScanProgressPayload | null>(null);
  const [pruneIgnored, setPruneIgnored] = useState(true);

  const [entries, setEntries] = useState<UnifiedEntry[]>([]);
  const [actionMap, setActionMap] = useState<Map<string, PlannedAction>>(new Map());

  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(new Set());
  const [onlyDiffs, setOnlyDiffs] = useState(true);

  // Virtual scrolling state
  const scrollContainerRef = useRef<HTMLDivElement>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewportHeight, setViewportHeight] = useState(600);

  // Diff drawer
  const [diffTarget, setDiffTarget] = useState<{
    path: string;
    contentA: string;
    contentB: string;
  } | null>(null);

  // Sync execution state
  const [isExecuting, setIsExecuting] = useState(false);
  const [syncProgress, setSyncProgress] = useState<SyncProgressPayload | null>(null);

  const { addNotification } = useNotification();

  // Listen to scan-progress and sync-progress
  useEffect(() => {
    let unlistenScan: (() => void) | undefined;
    let unlistenSync: (() => void) | undefined;

    listen<ScanProgressPayload>('scan-progress', (event) => {
      setScanProgress(event.payload);
    }).then((fn) => {
      unlistenScan = fn;
    });

    listen<SyncProgressPayload>('sync-progress', (event) => {
      setSyncProgress(event.payload);
    }).then((fn) => {
      unlistenSync = fn;
    });

    return () => {
      if (unlistenScan) unlistenScan();
      if (unlistenSync) unlistenSync();
    };
  }, []);

  // Update viewport height on resize
  useEffect(() => {
    const updateHeight = () => {
      if (scrollContainerRef.current) {
        setViewportHeight(scrollContainerRef.current.clientHeight);
      }
    };
    updateHeight();
    window.addEventListener('resize', updateHeight);
    return () => window.removeEventListener('resize', updateHeight);
  }, []);

  // Pick directory A
  const handleSelectDirA = async () => {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (selected && typeof selected === 'string') {
        await invoke('cancel_comparison');
        setDirA(selected);
      }
    } catch (e) {
      console.error(e);
      addNotification('error', 'Failed to pick directory');
    }
  };

  // Pick directory B
  const handleSelectDirB = async () => {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (selected && typeof selected === 'string') {
        await invoke('cancel_comparison');
        setDirB(selected);
      }
    } catch (e) {
      console.error(e);
      addNotification('error', 'Failed to pick directory');
    }
  };

  // Cancel in-flight scan
  const handleCancelScan = async () => {
    await invoke('cancel_comparison');
    setIsLoading(false);
    setScanProgress(null);
  };

  // Trigger compare
  const runCompare = useCallback(async () => {
    if (!dirA || !dirB) return;
    setIsLoading(true);
    setScanProgress({ count: 0, current_path: '', phase: 'Initializing' });

    try {
      const result: UnifiedEntry[] = await invoke('compare_directories', {
        dirA,
        dirB,
        pruneIgnored,
      });
      setEntries(result);
      setActionMap(new Map());

      // Auto expand folders with diffs
      const autoExpand = new Set<string>();
      for (const e of result) {
        if (e.diff_kind.type !== 'Identical') {
          const parts = e.relative_path.split('/');
          let acc = '';
          for (let i = 0; i < parts.length - 1; i++) {
            acc = acc ? `${acc}/${parts[i]}` : parts[i];
            autoExpand.add(acc);
          }
        }
      }
      setExpandedPaths(autoExpand);
    } catch (err) {
      const msg = String(err);
      if (!msg.includes('cancelled')) {
        console.error(err);
        addNotification('error', 'Comparison Failed', msg);
      }
    } finally {
      setIsLoading(false);
      setScanProgress(null);
    }
  }, [dirA, dirB, pruneIgnored, addNotification]);

  useEffect(() => {
    if (dirA && dirB) {
      runCompare();
    }
  }, [dirA, dirB, runCompare]);

  // Tree representation
  const tree = useMemo(() => buildTree(entries), [entries]);
  const rows: FlattenedRow[] = useMemo(
    () => flattenTree(tree, expandedPaths, onlyDiffs, actionMap),
    [tree, expandedPaths, onlyDiffs, actionMap]
  );

  // Virtual scrolling slice
  const { visibleRows, startIndex, totalHeight, paddingTop } = useMemo(() => {
    const count = rows.length;
    const total = count * ROW_HEIGHT;
    const start = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - BUFFER_SIZE);
    const visibleCount = Math.ceil(viewportHeight / ROW_HEIGHT) + 2 * BUFFER_SIZE;
    const end = Math.min(count, start + visibleCount);

    return {
      visibleRows: rows.slice(start, end),
      startIndex: start,
      totalHeight: total,
      paddingTop: start * ROW_HEIGHT,
    };
  }, [rows, scrollTop, viewportHeight]);

  const handleScroll = (e: React.UIEvent<HTMLDivElement>) => {
    setScrollTop(e.currentTarget.scrollTop);
  };

  // Statistics
  const stats = useMemo(() => {
    let diffCount = 0;
    let onlyACount = 0;
    let onlyBCount = 0;
    let modifiedCount = 0;
    for (const e of entries) {
      if (!e.is_dir) {
        if (e.diff_kind.type === 'OnlyInA') {
          onlyACount++;
          diffCount++;
        } else if (e.diff_kind.type === 'OnlyInB') {
          onlyBCount++;
          diffCount++;
        } else if (e.diff_kind.type === 'Modified') {
          modifiedCount++;
          diffCount++;
        }
      }
    }
    return { diffCount, onlyACount, onlyBCount, modifiedCount };
  }, [entries]);

  // Pending actions count
  const pendingActionsCount = useMemo(() => {
    let count = 0;
    for (const action of actionMap.values()) {
      if (action !== 'skip') count++;
    }
    return count;
  }, [actionMap]);

  // Toggle folder expansion
  const toggleExpand = (path: string) => {
    setExpandedPaths((prev) => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  };

  // Set action for an item
  const setAction = (path: string, action: PlannedAction) => {
    setActionMap((prev) => {
      const next = new Map(prev);
      if (action === 'skip') {
        next.delete(path);
      } else {
        next.set(path, action);
      }
      return next;
    });
  };

  // Batch Presets
  const applyPresetNewer = () => {
    const next = new Map<string, PlannedAction>();
    for (const e of entries) {
      if (e.is_dir) continue;
      if (e.diff_kind.type === 'OnlyInA') {
        next.set(e.relative_path, 'copy_a_to_b');
      } else if (e.diff_kind.type === 'OnlyInB') {
        next.set(e.relative_path, 'copy_b_to_a');
      } else if (e.diff_kind.type === 'Modified') {
        if (e.diff_kind.a_newer) {
          next.set(e.relative_path, 'copy_a_to_b');
        } else if (e.diff_kind.b_newer) {
          next.set(e.relative_path, 'copy_b_to_a');
        }
      }
    }
    setActionMap(next);
  };

  const applyPresetMirrorAtoB = () => {
    const next = new Map<string, PlannedAction>();
    for (const e of entries) {
      if (e.is_dir) continue;
      if (e.diff_kind.type === 'OnlyInA' || e.diff_kind.type === 'Modified') {
        next.set(e.relative_path, 'copy_a_to_b');
      } else if (e.diff_kind.type === 'OnlyInB') {
        next.set(e.relative_path, 'delete_b');
      }
    }
    setActionMap(next);
  };

  const applyPresetMirrorBtoA = () => {
    const next = new Map<string, PlannedAction>();
    for (const e of entries) {
      if (e.is_dir) continue;
      if (e.diff_kind.type === 'OnlyInB' || e.diff_kind.type === 'Modified') {
        next.set(e.relative_path, 'copy_b_to_a');
      } else if (e.diff_kind.type === 'OnlyInA') {
        next.set(e.relative_path, 'delete_a');
      }
    }
    setActionMap(next);
  };

  const resetActions = () => {
    setActionMap(new Map());
  };

  // Inspect file diff
  const handleOpenDiff = async (relPath: string) => {
    if (!dirA || !dirB) return;
    try {
      const [contentA, contentB]: [string, string] = await invoke('get_file_content_pair', {
        dirA,
        dirB,
        relativePath: relPath,
      });
      setDiffTarget({
        path: relPath,
        contentA,
        contentB,
      });
    } catch (e) {
      console.error(e);
      addNotification('error', 'Failed to read file contents');
    }
  };

  // Execute changes
  const handleExecute = async () => {
    if (!dirA || !dirB || pendingActionsCount === 0) return;
    setIsExecuting(true);
    setSyncProgress({ current: 0, total: pendingActionsCount, current_file: '' });

    const tasks: SyncTask[] = [];
    for (const [path, action] of actionMap.entries()) {
      if (action !== 'skip') {
        tasks.push({ relative_path: path, action });
      }
    }

    try {
      const count: number = await invoke('execute_sync', {
        dirA,
        dirB,
        tasks,
      });
      addNotification('success', 'Execution Completed', `Processed ${count} items`);
      setActionMap(new Map());
      await runCompare();
    } catch (err) {
      console.error(err);
      addNotification('error', 'Execution Failed', String(err));
    } finally {
      setIsExecuting(false);
      setSyncProgress(null);
    }
  };

  const handleBack = async () => {
    await invoke('cancel_comparison');
    onBack();
  };

  const dirNameA = dirA ? dirA.split(/[\\/]/).filter(Boolean).pop() || dirA : '';
  const dirNameB = dirB ? dirB.split(/[\\/]/).filter(Boolean).pop() || dirB : '';

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', width: '100%', position: 'relative' }}>
      {/* Top Toolbar */}
      <div style={{
        height: '44px',
        padding: '0 12px',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        borderBottom: '1px solid var(--border-subtle)',
        background: 'var(--bg-surface)',
        gap: '8px',
      }}>
        {/* Left: Navigation, Status & Summary */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px', minWidth: 0 }}>
          <Tooltip content="Back to Portal" position="bottom">
            <button className="icon-btn" onClick={handleBack}>
              <ArrowLeft size={15} />
            </button>
          </Tooltip>

          {dirA && dirB && (
            <Tooltip content={isLoading ? "Cancel Scan" : "Rescan"} position="bottom">
              <button
                className="icon-btn"
                onClick={isLoading ? handleCancelScan : runCompare}
                disabled={isExecuting}
                style={{ color: isLoading ? 'var(--status-danger)' : 'inherit' }}
              >
                {isLoading ? <X size={14} /> : <RefreshCw size={14} />}
              </button>
            </Tooltip>
          )}

          {/* Live scanning progress indicator */}
          {isLoading && scanProgress && (
            <div style={{ display: 'flex', alignItems: 'center', gap: '6px', marginLeft: '6px' }}>
              <Loader2 size={13} className="animate-spin" style={{ color: 'var(--status-info)' }} />
              <span style={{ fontSize: '12px', color: 'var(--text-secondary)', whiteSpace: 'nowrap' }}>
                {scanProgress.count.toLocaleString()} files ({scanProgress.phase})
              </span>
            </div>
          )}

          {/* Compact summary badges (when ready) */}
          {!isLoading && entries.length > 0 && (
            <div style={{ display: 'flex', alignItems: 'center', gap: '6px', marginLeft: '6px' }}>
              <span style={{
                fontSize: '11px',
                fontWeight: 600,
                padding: '2px 8px',
                borderRadius: '100px',
                background: stats.diffCount > 0 ? 'var(--status-warning-light)' : 'var(--status-success-light)',
                color: stats.diffCount > 0 ? 'var(--status-warning)' : 'var(--status-success)',
              }}>
                {stats.diffCount} diffs
              </span>
              <span style={{ fontSize: '11px', padding: '2px 6px', borderRadius: '4px', background: 'var(--bg-surface-hover)', color: 'var(--text-secondary)' }}>
                +{stats.onlyACount} A
              </span>
              <span style={{ fontSize: '11px', padding: '2px 6px', borderRadius: '4px', background: 'var(--bg-surface-hover)', color: 'var(--text-secondary)' }}>
                +{stats.onlyBCount} B
              </span>
              <span style={{ fontSize: '11px', padding: '2px 6px', borderRadius: '4px', background: 'var(--bg-surface-hover)', color: 'var(--text-secondary)' }}>
                ~{stats.modifiedCount} mod
              </span>
            </div>
          )}
        </div>

        {/* Right: Prune Toggle, Filter & Batch Actions */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
          {/* Smart Pruning Toggle */}
          <Tooltip content={pruneIgnored ? "Smart Prune: ON (Skips node_modules/target/git)" : "Smart Prune: OFF (Scan all files)"} position="bottom">
            <button
              className="icon-btn"
              onClick={() => setPruneIgnored(!pruneIgnored)}
              style={{ color: pruneIgnored ? 'var(--status-success)' : 'var(--status-warning)' }}
            >
              {pruneIgnored ? <Shield size={14} /> : <ShieldAlert size={14} />}
            </button>
          </Tooltip>

          {entries.length > 0 && (
            <>
              <Tooltip content={onlyDiffs ? "Show All Items" : "Show Only Differences"} position="bottom">
                <button
                  className="icon-btn"
                  onClick={() => setOnlyDiffs(!onlyDiffs)}
                  style={{ color: onlyDiffs ? 'var(--status-info)' : 'var(--text-secondary)' }}
                >
                  <Filter size={14} />
                </button>
              </Tooltip>

              <div style={{ width: '1px', height: '18px', background: 'var(--border-default)', margin: '0 4px' }} />

              <Tooltip content="Sync Newer Files" position="bottom">
                <button className="icon-btn" onClick={applyPresetNewer} disabled={isExecuting || isLoading}>
                  <Sparkles size={14} />
                </button>
              </Tooltip>

              <Tooltip content="Mirror A to B" position="bottom">
                <button className="icon-btn" onClick={applyPresetMirrorAtoB} disabled={isExecuting || isLoading}>
                  <ArrowRightToLine size={14} />
                </button>
              </Tooltip>

              <Tooltip content="Mirror B to A" position="bottom">
                <button className="icon-btn" onClick={applyPresetMirrorBtoA} disabled={isExecuting || isLoading}>
                  <ArrowLeftToLine size={14} />
                </button>
              </Tooltip>

              <Tooltip content="Reset All Actions" position="bottom">
                <button className="icon-btn" onClick={resetActions} disabled={isExecuting || isLoading || pendingActionsCount === 0}>
                  <RotateCcw size={14} />
                </button>
              </Tooltip>
            </>
          )}
        </div>
      </div>

      {/* Comparison Workspace (Dual-Pane) */}
      <div style={{ display: 'flex', flex: 1, overflow: 'hidden', position: 'relative' }}>
        {/* If one or both directories not selected -> Empty State Prompts */}
        {(!dirA || !dirB) ? (
          <div style={{ display: 'flex', flex: 1, padding: '16px', gap: '16px', background: 'var(--bg-base)' }}>
            <div style={{ flex: 1, display: 'flex', flexDirection: 'column' }}>
              {dirA ? (
                <SelectedPaneHeader
                  label="Directory A"
                  dirPath={dirA}
                  onReselect={handleSelectDirA}
                />
              ) : (
                <EmptyPaneSlot
                  label="Select Directory A"
                  onClick={handleSelectDirA}
                />
              )}
            </div>

            <div style={{ flex: 1, display: 'flex', flexDirection: 'column' }}>
              {dirB ? (
                <SelectedPaneHeader
                  label="Directory B"
                  dirPath={dirB}
                  onReselect={handleSelectDirB}
                />
              ) : (
                <EmptyPaneSlot
                  label="Select Directory B"
                  onClick={handleSelectDirB}
                />
              )}
            </div>
          </div>
        ) : (
          /* Both Selected: Dual-Pane Synchronized Virtual Tree */
          <div style={{ display: 'flex', flexDirection: 'column', flex: 1, overflow: 'hidden' }}>
            {/* Active Headers */}
            <div style={{
              display: 'grid',
              gridTemplateColumns: '1fr 140px 1fr',
              borderBottom: '1px solid var(--border-subtle)',
              background: 'var(--bg-surface)',
              alignItems: 'center',
            }}>
              <ActiveHeaderBar
                label="Directory A"
                dirPath={dirA}
                onReselect={handleSelectDirA}
              />
              <div style={{ textAlign: 'center', fontSize: '11px', fontWeight: 600, color: 'var(--text-tertiary)' }}>
                Action
              </div>
              <ActiveHeaderBar
                label="Directory B"
                dirPath={dirB}
                onReselect={handleSelectDirB}
              />
            </div>

            {/* Virtual Scroll Container */}
            <div
              ref={scrollContainerRef}
              onScroll={handleScroll}
              style={{
                flex: 1,
                overflowY: 'auto',
                overflowX: 'hidden',
                position: 'relative',
              }}
            >
              {isLoading && rows.length === 0 ? (
                <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', height: '100%', gap: '10px', color: 'var(--text-secondary)' }}>
                  <Loader2 size={24} className="animate-spin" style={{ color: 'var(--status-info)' }} />
                  {scanProgress && (
                    <span style={{ fontSize: '12px' }}>
                      {scanProgress.count.toLocaleString()} files scanned ({scanProgress.phase})
                    </span>
                  )}
                </div>
              ) : rows.length === 0 ? (
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', color: 'var(--text-tertiary)', fontSize: '13px' }}>
                  {onlyDiffs ? 'Identical (0 differences)' : 'Empty directory'}
                </div>
              ) : (
                <div style={{ height: `${totalHeight}px`, position: 'relative' }}>
                  <div style={{ transform: `translateY(${paddingTop}px)` }}>
                    {visibleRows.map((row, index) => (
                      <DualTreeRow
                        key={row.node.relative_path || startIndex + index}
                        row={row}
                        action={actionMap.get(row.node.relative_path) || 'skip'}
                        onToggleExpand={() => toggleExpand(row.node.relative_path)}
                        onSetAction={(act) => setAction(row.node.relative_path, act)}
                        onOpenDiff={() => handleOpenDiff(row.node.relative_path)}
                      />
                    ))}
                  </div>
                </div>
              )}
            </div>
          </div>
        )}

        {/* Diff Drawer */}
        {diffTarget && (
          <DiffDrawer
            relativePath={diffTarget.path}
            contentA={diffTarget.contentA}
            contentB={diffTarget.contentB}
            dirNameA={dirNameA}
            dirNameB={dirNameB}
            onClose={() => setDiffTarget(null)}
          />
        )}
      </div>

      {/* Floating Execution Bar (Only when pending actions > 0) */}
      {pendingActionsCount > 0 && (
        <div style={{
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
        }}>
          {isExecuting && syncProgress ? (
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px', fontSize: '12px', color: 'var(--text-secondary)' }}>
              <Loader2 size={14} className="animate-spin" style={{ color: 'var(--status-info)' }} />
              <span>{syncProgress.current} / {syncProgress.total}</span>
            </div>
          ) : (
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <span style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-primary)' }}>
                {pendingActionsCount} planned
              </span>
              <Tooltip content="Execute Planned Changes" position="top">
                <button
                  className="icon-btn"
                  onClick={handleExecute}
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
      )}
    </div>
  );
}

// Subcomponents: EmptyPaneSlot
function EmptyPaneSlot({ label, onClick }: { label: string; onClick: () => void }) {
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
}

// Subcomponents: SelectedPaneHeader
function SelectedPaneHeader({
  label,
  dirPath,
  onReselect,
}: {
  label: string;
  dirPath: string;
  onReselect: () => void;
}) {
  const folderName = dirPath.split(/[\\/]/).filter(Boolean).pop() || dirPath;
  return (
    <div style={{
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'space-between',
      padding: '8px 12px',
      background: 'var(--bg-surface)',
      borderRadius: 'var(--radius-sm)',
      border: '1px solid var(--border-subtle)',
    }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: '8px', minWidth: 0 }}>
        <Folder size={16} style={{ color: 'var(--status-info)', flexShrink: 0 }} />
        <div style={{ minWidth: 0 }}>
          <div style={{ fontSize: '13px', fontWeight: 600, color: 'var(--text-primary)' }}>
            {folderName}
          </div>
          <div style={{ fontSize: '11px', color: 'var(--text-secondary)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
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
}

// Subcomponents: ActiveHeaderBar
function ActiveHeaderBar({
  dirPath,
  onReselect,
}: {
  label: string;
  dirPath: string;
  onReselect: () => void;
}) {
  const folderName = dirPath.split(/[\\/]/).filter(Boolean).pop() || dirPath;
  return (
    <div style={{
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'space-between',
      padding: '6px 12px',
      minWidth: 0,
    }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: '6px', minWidth: 0 }}>
        <Folder size={14} style={{ color: 'var(--status-info)', flexShrink: 0 }} />
        <span style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-primary)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }} title={dirPath}>
          {folderName}
        </span>
      </div>
      <div style={{ display: 'flex', alignItems: 'center', gap: '2px', flexShrink: 0 }}>
        <Tooltip content="Open in Explorer" position="bottom">
          <button className="icon-btn" onClick={() => invoke('reveal_in_explorer', { fullPath: dirPath })}>
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
}

// Subcomponents: DualTreeRow (Direct Visual Preview with virtual row height)
function DualTreeRow({
  row,
  action,
  onToggleExpand,
  onSetAction,
  onOpenDiff,
}: {
  row: FlattenedRow;
  action: PlannedAction;
  onToggleExpand: () => void;
  onSetAction: (act: PlannedAction) => void;
  onOpenDiff: () => void;
}) {
  const { node, depth, hasChildren, isExpanded } = row;

  const existsInA = Boolean(node.meta_a);
  const existsInB = Boolean(node.meta_b);

  const isGhostA = !existsInA && action === 'copy_b_to_a';
  const isDeletedA = existsInA && action === 'delete_a';
  const isTransferredAtoB = existsInA && action === 'copy_a_to_b';
  const isOverwrittenByB = existsInA && action === 'copy_b_to_a';

  const isGhostB = !existsInB && action === 'copy_a_to_b';
  const isDeletedB = existsInB && action === 'delete_b';
  const isTransferredBtoA = existsInB && action === 'copy_b_to_a';
  const isOverwrittenByA = existsInB && action === 'copy_a_to_b';

  return (
    <div style={{
      display: 'grid',
      gridTemplateColumns: '1fr 140px 1fr',
      height: `${ROW_HEIGHT}px`,
      borderBottom: '1px solid var(--border-subtle)',
      alignItems: 'center',
      fontSize: '12px',
      boxSizing: 'border-box',
    }}
    onMouseEnter={(e) => { e.currentTarget.style.backgroundColor = 'var(--bg-surface-hover)'; }}
    onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = 'transparent'; }}
    >
      {/* Left Node: Side A */}
      <div style={{
        paddingLeft: `${depth * 18 + 8}px`,
        display: 'flex',
        alignItems: 'center',
        gap: '6px',
        overflow: 'hidden',
        minWidth: 0,
        opacity: isDeletedA ? 0.45 : isGhostA ? 0.75 : 1,
        textDecoration: isDeletedA ? 'line-through' : 'none',
      }}>
        {hasChildren ? (
          <button
            onClick={onToggleExpand}
            style={{ background: 'transparent', border: 'none', cursor: 'pointer', padding: 0, color: 'var(--text-tertiary)', display: 'flex' }}
          >
            {isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
          </button>
        ) : (
          <span style={{ width: 14 }} />
        )}

        {node.is_dir ? (
          <Folder size={14} style={{ color: 'var(--text-tertiary)', flexShrink: 0 }} />
        ) : (
          <FileText size={14} style={{ color: 'var(--text-secondary)', flexShrink: 0 }} />
        )}

        {existsInA ? (
          <span
            style={{
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
              color: isDeletedA ? 'var(--status-danger)' : 'var(--text-primary)',
              fontWeight: existsInA && !existsInB ? 600 : 400,
            }}
          >
            {node.name}
          </span>
        ) : isGhostA ? (
          <span style={{
            fontStyle: 'italic',
            color: 'var(--status-success)',
            borderBottom: '1px dashed var(--status-success)',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}>
            + {node.name}
          </span>
        ) : (
          <span style={{ color: 'var(--text-tertiary)', fontStyle: 'italic' }}>—</span>
        )}

        {isTransferredAtoB && (
          <span style={{ fontSize: '10px', padding: '1px 4px', borderRadius: '4px', background: 'var(--status-info-light)', color: 'var(--status-info)', flexShrink: 0 }}>
            ➔ B
          </span>
        )}
        {isOverwrittenByB && (
          <span style={{ fontSize: '10px', padding: '1px 4px', borderRadius: '4px', background: 'var(--status-warning-light)', color: 'var(--status-warning)', flexShrink: 0 }}>
            ⚠ Overwrite
          </span>
        )}
        {isDeletedA && (
          <span style={{ fontSize: '10px', padding: '1px 4px', borderRadius: '4px', background: 'var(--status-danger-light)', color: 'var(--status-danger)', flexShrink: 0 }}>
            ✕ Del
          </span>
        )}

        {existsInA && !node.is_dir && (
          <Tooltip content={`Modified: ${formatTimestamp(node.meta_a?.modified_timestamp)}`} position="top">
            <span style={{ fontSize: '11px', color: 'var(--text-tertiary)', marginLeft: 'auto', paddingRight: '8px', flexShrink: 0, cursor: 'default' }}>
              {formatFileSize(node.meta_a?.size)}
            </span>
          </Tooltip>
        )}
      </div>

      {/* Center Action Column */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '3px' }}>
        {node.is_dir ? (
          <span style={{ width: 14 }} />
        ) : node.diff_kind.type === 'Identical' ? (
          <Check size={12} style={{ color: 'var(--status-success)', opacity: 0.6 }} />
        ) : (
          <>
            <Tooltip content="Copy A ➔ B" position="top">
              <button
                className="icon-btn"
                onClick={() => onSetAction(action === 'copy_a_to_b' ? 'skip' : 'copy_a_to_b')}
                style={{
                  width: '24px',
                  height: '24px',
                  background: action === 'copy_a_to_b' ? 'var(--status-info)' : 'transparent',
                  color: action === 'copy_a_to_b' ? '#ffffff' : 'var(--text-secondary)',
                }}
              >
                <ArrowRight size={12} />
              </button>
            </Tooltip>

            <Tooltip content="Copy B ⬅ A" position="top">
              <button
                className="icon-btn"
                onClick={() => onSetAction(action === 'copy_b_to_a' ? 'skip' : 'copy_b_to_a')}
                style={{
                  width: '24px',
                  height: '24px',
                  background: action === 'copy_b_to_a' ? 'var(--status-info)' : 'transparent',
                  color: action === 'copy_b_to_a' ? '#ffffff' : 'var(--text-secondary)',
                }}
              >
                <ArrowLeftIcon size={12} />
              </button>
            </Tooltip>

            <Tooltip content="Delete" position="top">
              <button
                className="icon-btn danger"
                onClick={() => {
                  if (existsInA && !existsInB) {
                    onSetAction(action === 'delete_a' ? 'skip' : 'delete_a');
                  } else if (existsInB && !existsInA) {
                    onSetAction(action === 'delete_b' ? 'skip' : 'delete_b');
                  } else {
                    onSetAction(action === 'delete_b' ? 'skip' : 'delete_b');
                  }
                }}
                style={{
                  width: '24px',
                  height: '24px',
                  background: action === 'delete_a' || action === 'delete_b' ? 'var(--status-danger)' : 'transparent',
                  color: action === 'delete_a' || action === 'delete_b' ? '#ffffff' : 'var(--text-secondary)',
                }}
              >
                <Trash2 size={12} />
              </button>
            </Tooltip>

            <Tooltip content="Skip" position="top">
              <button
                className="icon-btn"
                onClick={() => onSetAction('skip')}
                style={{
                  width: '24px',
                  height: '24px',
                  color: action === 'skip' ? 'var(--text-tertiary)' : 'var(--text-secondary)',
                }}
              >
                <Minus size={12} />
              </button>
            </Tooltip>

            {node.diff_kind.type === 'Modified' && (
              <Tooltip content="Inspect Diff" position="top">
                <button
                  className="icon-btn"
                  onClick={onOpenDiff}
                  style={{ width: '24px', height: '24px', color: 'var(--status-info)' }}
                >
                  <FileCode size={13} />
                </button>
              </Tooltip>
            )}
          </>
        )}
      </div>

      {/* Right Node: Side B */}
      <div style={{
        paddingLeft: `${depth * 18 + 8}px`,
        display: 'flex',
        alignItems: 'center',
        gap: '6px',
        overflow: 'hidden',
        minWidth: 0,
        opacity: isDeletedB ? 0.45 : isGhostB ? 0.75 : 1,
        textDecoration: isDeletedB ? 'line-through' : 'none',
      }}>
        <span style={{ width: 14 }} />

        {node.is_dir ? (
          <Folder size={14} style={{ color: 'var(--text-tertiary)', flexShrink: 0 }} />
        ) : (
          <FileText size={14} style={{ color: 'var(--text-secondary)', flexShrink: 0 }} />
        )}

        {existsInB ? (
          <span
            style={{
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
              color: isDeletedB ? 'var(--status-danger)' : 'var(--text-primary)',
              fontWeight: existsInB && !existsInA ? 600 : 400,
            }}
          >
            {node.name}
          </span>
        ) : isGhostB ? (
          <span style={{
            fontStyle: 'italic',
            color: 'var(--status-success)',
            borderBottom: '1px dashed var(--status-success)',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}>
            + {node.name}
          </span>
        ) : (
          <span style={{ color: 'var(--text-tertiary)', fontStyle: 'italic' }}>—</span>
        )}

        {isTransferredBtoA && (
          <span style={{ fontSize: '10px', padding: '1px 4px', borderRadius: '4px', background: 'var(--status-info-light)', color: 'var(--status-info)', flexShrink: 0 }}>
            ⬅ A
          </span>
        )}
        {isOverwrittenByA && (
          <span style={{ fontSize: '10px', padding: '1px 4px', borderRadius: '4px', background: 'var(--status-warning-light)', color: 'var(--status-warning)', flexShrink: 0 }}>
            ⚠ Overwrite
          </span>
        )}
        {isDeletedB && (
          <span style={{ fontSize: '10px', padding: '1px 4px', borderRadius: '4px', background: 'var(--status-danger-light)', color: 'var(--status-danger)', flexShrink: 0 }}>
            ✕ Del
          </span>
        )}

        {existsInB && !node.is_dir && (
          <Tooltip content={`Modified: ${formatTimestamp(node.meta_b?.modified_timestamp)}`} position="top">
            <span style={{ fontSize: '11px', color: 'var(--text-tertiary)', marginLeft: 'auto', paddingRight: '8px', flexShrink: 0, cursor: 'default' }}>
              {formatFileSize(node.meta_b?.size)}
            </span>
          </Tooltip>
        )}
      </div>
    </div>
  );
}
