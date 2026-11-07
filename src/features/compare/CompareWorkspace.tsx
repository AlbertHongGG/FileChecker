import { useCallback } from 'react';
import { useDirectoryScan } from './hooks/useDirectoryScan';
import { useSyncPlanner } from './hooks/useSyncPlanner';
import { useTreeHierarchy } from './hooks/useTreeHierarchy';
import { useVirtualScroll } from './hooks/useVirtualScroll';
import { useFileDiff } from '../diff/hooks/useFileDiff';
import { CompareToolbar } from './components/CompareToolbar';
import { WorkspaceHeaderRow } from './components/DirectorySelector';
import { SplitPaneStage } from './components/SplitPaneStage';
import { DualTreeVirtualList } from './components/tree/DualTreeVirtualList';
import { ExecutionBar } from './components/ExecutionBar';
import { DiffDrawer } from '../diff/components/DiffDrawer';
import { collectSubtreeDescendants } from './utils/subtreeQuery';

interface CompareWorkspaceProps {
  onBack: () => void;
}

export default function CompareWorkspace({ onBack }: CompareWorkspaceProps) {
  // 1. Directory scan lifecycle
  const {
    dirA,
    dirB,
    selectDirA,
    selectDirB,
    isLoading,
    scanProgress,
    pruneIgnored,
    setPruneIgnored,
    entries,
    scanVersion,
    applySyncPatch,
    runCompare,
    cancelScan,
  } = useDirectoryScan();

  // 2. Action planning & sync execution
  const {
    actionMap,
    setAction,
    setDirectoryAction,
    applyPresetNewer,
    applyPresetMirrorAtoB,
    applyPresetMirrorBtoA,
    resetActions,
    stats,
    pendingActionsCount,
    isExecuting,
    syncProgress,
    execute,
  } = useSyncPlanner(entries, scanVersion);

  // 3. Tree hierarchy & filtering
  const {
    rows,
    toggleExpand,
    onlyDiffs,
    setOnlyDiffs,
  } = useTreeHierarchy(entries, scanVersion);

  // 4. Virtual list rendering slice
  const {
    scrollContainerRef,
    handleScroll,
    visibleRows,
    totalHeight,
    paddingTop,
  } = useVirtualScroll(rows);

  // 5. File pair diff viewer drawer
  const {
    diffSession,
    isLoadingDiff,
    openDiff,
    selectDiffFile,
    closeDiff,
  } = useFileDiff(dirA, dirB);

  const handleBack = useCallback(async () => {
    await cancelScan();
    onBack();
  }, [cancelScan, onBack]);

  const handleExecute = useCallback(() => {
    if (!dirA || !dirB) return;
    execute(dirA, dirB, applySyncPatch);
  }, [dirA, dirB, execute, applySyncPatch]);

  const dirNameA = dirA ? dirA.split(/[\\/]/).filter(Boolean).pop() || dirA : '';
  const dirNameB = dirB ? dirB.split(/[\\/]/).filter(Boolean).pop() || dirB : '';

  // Only expand to 3-column with Action when comparison is ready and tree can be rendered
  const isTreeReady = Boolean(dirA && dirB && !isLoading && entries.length > 0);

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        width: '100%',
        position: 'relative',
        background: 'var(--bg-base)',
      }}
    >
      {/* 1. Top Global Navigation & Action Toolbar */}
      <CompareToolbar
        onBack={handleBack}
        dirA={dirA}
        dirB={dirB}
        isLoading={isLoading}
        isExecuting={isExecuting}
        entriesCount={entries.length}
        stats={stats}
        pruneIgnored={pruneIgnored}
        onTogglePrune={() => setPruneIgnored(!pruneIgnored)}
        onlyDiffs={onlyDiffs}
        onToggleOnlyDiffs={() => setOnlyDiffs(!onlyDiffs)}
        onApplyPresetNewer={applyPresetNewer}
        onApplyPresetMirrorAtoB={applyPresetMirrorAtoB}
        onApplyPresetMirrorBtoA={applyPresetMirrorBtoA}
        onResetActions={resetActions}
        pendingActionsCount={pendingActionsCount}
        onRescan={runCompare}
        onCancelScan={cancelScan}
      />

      {/* 2. Dynamic Header: 2-Column during Selection/Scan, expands to 3-Column on Tree Ready */}
      <WorkspaceHeaderRow
        dirA={dirA}
        dirB={dirB}
        showActionColumn={isTreeReady}
        isLoading={isLoading}
        isExecuting={isExecuting}
        onSelectDirA={selectDirA}
        onSelectDirB={selectDirB}
      />

      {/* 3. Main Stage Area: 2-Column SplitPaneStage vs 3-Column DualTreeVirtualList */}
      <div style={{ display: 'flex', flex: 1, overflow: 'hidden', position: 'relative' }}>
        {!isTreeReady ? (
          <SplitPaneStage
            dirA={dirA}
            dirB={dirB}
            isLoading={isLoading}
            scanProgress={scanProgress}
            onSelectDirA={selectDirA}
            onSelectDirB={selectDirB}
          />
        ) : (
          <DualTreeVirtualList
            scrollContainerRef={scrollContainerRef}
            onScroll={handleScroll}
            onlyDiffs={onlyDiffs}
            rowsCount={rows.length}
            visibleRows={visibleRows}
            totalHeight={totalHeight}
            paddingTop={paddingTop}
            actionMap={actionMap}
            onToggleExpand={toggleExpand}
            onSetAction={setAction}
            onSetDirectoryAction={(node, actionType) => {
              const descendants = collectSubtreeDescendants(node);
              setDirectoryAction(descendants, actionType);
            }}
            onOpenDiff={openDiff}
          />
        )}

        {/* Diff Drawer Modal */}
        {diffSession && (
          <DiffDrawer
            session={diffSession}
            isLoading={isLoadingDiff}
            dirNameA={dirNameA}
            dirNameB={dirNameB}
            onSelectFile={selectDiffFile}
            onClose={closeDiff}
          />
        )}
      </div>

      {/* 4. Floating Execution Bar */}
      <ExecutionBar
        pendingActionsCount={pendingActionsCount}
        isExecuting={isExecuting}
        syncProgress={syncProgress}
        onExecute={handleExecute}
      />
    </div>
  );
}
