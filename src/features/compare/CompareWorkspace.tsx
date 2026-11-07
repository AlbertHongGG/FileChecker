import { useCallback } from 'react';
import { useDirectoryScan } from './hooks/useDirectoryScan';
import { useSyncPlanner } from './hooks/useSyncPlanner';
import { useTreeHierarchy } from './hooks/useTreeHierarchy';
import { useVirtualScroll } from './hooks/useVirtualScroll';
import { useFileDiff } from '../diff/hooks/useFileDiff';
import { CompareToolbar } from './components/CompareToolbar';
import {
  EmptyPaneSlot,
  SelectedPaneHeader,
  ActiveHeaderBar,
} from './components/DirectorySelector';
import { DualTreeVirtualList } from './components/tree/DualTreeVirtualList';
import { ExecutionBar } from './components/ExecutionBar';
import { DiffDrawer } from '../diff/components/DiffDrawer';

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

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        width: '100%',
        position: 'relative',
      }}
    >
      {/* Top Toolbar */}
      <CompareToolbar
        onBack={handleBack}
        dirA={dirA}
        dirB={dirB}
        isLoading={isLoading}
        isExecuting={isExecuting}
        scanProgress={scanProgress}
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

      {/* Main Dual-Pane Workspace */}
      <div style={{ display: 'flex', flex: 1, overflow: 'hidden', position: 'relative' }}>
        {!dirA || !dirB ? (
          /* Empty / Incomplete selection slots */
          <div
            style={{
              display: 'flex',
              flex: 1,
              padding: '16px',
              gap: '16px',
              background: 'var(--bg-base)',
            }}
          >
            <div style={{ flex: 1, display: 'flex', flexDirection: 'column' }}>
              {dirA ? (
                <SelectedPaneHeader label="Directory A" dirPath={dirA} onReselect={selectDirA} />
              ) : (
                <EmptyPaneSlot label="Select Directory A" onClick={selectDirA} />
              )}
            </div>

            <div style={{ flex: 1, display: 'flex', flexDirection: 'column' }}>
              {dirB ? (
                <SelectedPaneHeader label="Directory B" dirPath={dirB} onReselect={selectDirB} />
              ) : (
                <EmptyPaneSlot label="Select Directory B" onClick={selectDirB} />
              )}
            </div>
          </div>
        ) : (
          /* Dual-Pane Synchronized Virtual Tree */
          <div style={{ display: 'flex', flexDirection: 'column', flex: 1, overflow: 'hidden' }}>
            <div
              style={{
                display: 'grid',
                gridTemplateColumns: '1fr 140px 1fr',
                borderBottom: '1px solid var(--border-subtle)',
                background: 'var(--bg-surface)',
                alignItems: 'center',
              }}
            >
              <ActiveHeaderBar label="Directory A" dirPath={dirA} onReselect={selectDirA} />
              <div
                style={{
                  textAlign: 'center',
                  fontSize: '11px',
                  fontWeight: 600,
                  color: 'var(--text-tertiary)',
                }}
              >
                Action
              </div>
              <ActiveHeaderBar label="Directory B" dirPath={dirB} onReselect={selectDirB} />
            </div>

            <DualTreeVirtualList
              scrollContainerRef={scrollContainerRef}
              onScroll={handleScroll}
              isLoading={isLoading}
              scanProgress={scanProgress}
              onlyDiffs={onlyDiffs}
              rowsCount={rows.length}
              visibleRows={visibleRows}
              totalHeight={totalHeight}
              paddingTop={paddingTop}
              actionMap={actionMap}
              onToggleExpand={toggleExpand}
              onSetAction={setAction}
              onSetDirectoryAction={(node, actionType) => {
                if (node.descendantEntries) {
                  setDirectoryAction(node.descendantEntries, actionType);
                }
              }}
              onOpenDiff={openDiff}
              onCancelScan={cancelScan}
            />
          </div>
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

      {/* Floating Execution Bar */}
      <ExecutionBar
        pendingActionsCount={pendingActionsCount}
        isExecuting={isExecuting}
        syncProgress={syncProgress}
        onExecute={handleExecute}
      />
    </div>
  );
}
