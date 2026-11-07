import { useState, useEffect, useMemo, useCallback } from 'react';
import { TauriClient } from '../../../services/tauriClient';
import { EventListenerService } from '../../../services/eventListener';
import {
  PlannedAction,
  UnifiedEntry,
  SyncTask,
  SyncProgressPayload,
  DescendantEntry,
} from '../../../types';
import { useNotification } from '../../../contexts/NotificationContext';

export function useSyncPlanner(entries: UnifiedEntry[], scanVersion: number) {
  const [actionMap, setActionMap] = useState<Map<string, PlannedAction>>(
    new Map()
  );
  const [isExecuting, setIsExecuting] = useState(false);
  const [syncProgress, setSyncProgress] = useState<SyncProgressPayload | null>(
    null
  );

  const { addNotification } = useNotification();

  // Reset action map when a fresh scan completes
  useEffect(() => {
    setActionMap(new Map());
  }, [scanVersion]);

  // Listen to sync-progress event
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    EventListenerService.onSyncProgress((payload) => {
      setSyncProgress(payload);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  const setAction = useCallback((path: string, action: PlannedAction) => {
    setActionMap((prev) => {
      const next = new Map(prev);
      if (action === 'skip') {
        next.delete(path);
      } else {
        next.set(path, action);
      }
      return next;
    });
  }, []);

  const setDirectoryAction = useCallback(
    (
      targetEntries: DescendantEntry[],
      actionType: 'copy_a_to_b' | 'copy_b_to_a' | 'delete' | 'skip'
    ) => {
      setActionMap((prev) => {
        const next = new Map(prev);

        if (actionType === 'skip') {
          for (const item of targetEntries) {
            next.delete(item.path);
          }
          return next;
        }

        if (actionType === 'copy_a_to_b') {
          // All items present in A or modified
          const eligible = targetEntries.filter(
            (item) => item.diffType === 'OnlyInA' || item.diffType === 'Modified'
          );
          const allActive =
            eligible.length > 0 &&
            eligible.every((item) => next.get(item.path) === 'copy_a_to_b');

          for (const item of eligible) {
            if (allActive) {
              next.delete(item.path);
            } else {
              next.set(item.path, 'copy_a_to_b');
            }
          }
          return next;
        }

        if (actionType === 'copy_b_to_a') {
          // All items present in B or modified
          const eligible = targetEntries.filter(
            (item) => item.diffType === 'OnlyInB' || item.diffType === 'Modified'
          );
          const allActive =
            eligible.length > 0 &&
            eligible.every((item) => next.get(item.path) === 'copy_b_to_a');

          for (const item of eligible) {
            if (allActive) {
              next.delete(item.path);
            } else {
              next.set(item.path, 'copy_b_to_a');
            }
          }
          return next;
        }

        if (actionType === 'delete') {
          // All items with any difference (OnlyInA, OnlyInB, or Modified)
          const eligible = targetEntries.filter((item) => item.diffType !== 'Identical');
          const allActive =
            eligible.length > 0 &&
            eligible.every((item) => {
              const current = next.get(item.path);
              return (
                current === 'delete_a' ||
                current === 'delete_b' ||
                current === 'delete_both'
              );
            });

          for (const item of eligible) {
            if (allActive) {
              next.delete(item.path);
            } else {
              if (item.diffType === 'OnlyInA') {
                next.set(item.path, 'delete_a');
              } else if (item.diffType === 'OnlyInB') {
                next.set(item.path, 'delete_b');
              } else {
                next.set(item.path, 'delete_both');
              }
            }
          }
          return next;
        }

        return next;
      });
    },
    []
  );

  const applyPresetNewer = useCallback(() => {
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
  }, [entries]);

  const applyPresetMirrorAtoB = useCallback(() => {
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
  }, [entries]);

  const applyPresetMirrorBtoA = useCallback(() => {
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
  }, [entries]);

  const resetActions = useCallback(() => {
    setActionMap(new Map());
  }, []);

  // Compute stats
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

  const pendingActionsCount = useMemo(() => {
    let count = 0;
    for (const action of actionMap.values()) {
      if (action !== 'skip') count++;
    }
    return count;
  }, [actionMap]);

  const execute = useCallback(
    async (
      dirA: string,
      dirB: string,
      onSuccess?: (tasks: SyncTask[]) => void
    ) => {
      if (!dirA || !dirB || pendingActionsCount === 0) return;
      setIsExecuting(true);
      setSyncProgress({
        current: 0,
        total: pendingActionsCount,
        current_file: '',
      });

      const tasks: SyncTask[] = [];
      for (const [path, action] of actionMap.entries()) {
        if (action !== 'skip') {
          tasks.push({ relative_path: path, action });
        }
      }

      try {
        const count = await TauriClient.executeSync(dirA, dirB, tasks);
        addNotification(
          'success',
          'Execution Completed',
          `Processed ${count} items`
        );
        setActionMap(new Map());
        if (onSuccess) {
          onSuccess(tasks);
        }
      } catch (err) {
        console.error('Execution Failed:', err);
        addNotification('error', 'Execution Failed', String(err));
      } finally {
        setIsExecuting(false);
        setSyncProgress(null);
      }
    },
    [pendingActionsCount, actionMap, addNotification]
  );

  return {
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
  };
}
