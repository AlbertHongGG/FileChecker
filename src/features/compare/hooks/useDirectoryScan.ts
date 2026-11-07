import { useState, useEffect, useCallback } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { TauriClient } from '../../../services/tauriClient';
import { EventListenerService } from '../../../services/eventListener';
import { UnifiedEntry, ScanProgressPayload, SyncTask, PlannedAction } from '../../../types';
import { useNotification } from '../../../contexts/NotificationContext';

export function useDirectoryScan() {
  const [dirA, setDirA] = useState<string | null>(null);
  const [dirB, setDirB] = useState<string | null>(null);

  const [isLoading, setIsLoading] = useState(false);
  const [scanProgress, setScanProgress] = useState<ScanProgressPayload | null>(null);
  const [pruneIgnored, setPruneIgnored] = useState(true);
  const [entries, setEntries] = useState<UnifiedEntry[]>([]);

  const { addNotification } = useNotification();

  // Listen to scan-progress event
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    EventListenerService.onScanProgress((payload) => {
      setScanProgress(payload);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  const cancelScan = useCallback(async () => {
    await TauriClient.cancelComparison();
    setIsLoading(false);
    setScanProgress(null);
  }, []);

  const selectDirA = useCallback(async () => {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (selected && typeof selected === 'string') {
        await TauriClient.cancelComparison();
        setDirA(selected);
      }
    } catch (e) {
      console.error(e);
      addNotification('error', 'Selection Failed', 'Failed to pick directory A');
    }
  }, [addNotification]);

  const selectDirB = useCallback(async () => {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (selected && typeof selected === 'string') {
        await TauriClient.cancelComparison();
        setDirB(selected);
      }
    } catch (e) {
      console.error(e);
      addNotification('error', 'Selection Failed', 'Failed to pick directory B');
    }
  }, [addNotification]);

  const runCompare = useCallback(async () => {
    if (!dirA || !dirB) return;
    setIsLoading(true);
    setScanProgress({
      completed_a: 0,
      total_a: 0,
      current_name_a: '',
      count_a: 0,
      is_hdd_a: false,
      completed_b: 0,
      total_b: 0,
      current_name_b: '',
      count_b: 0,
      is_hdd_b: false,
      phase: 'Initializing',
    });

    try {
      const result = await TauriClient.compareDirectories(
        dirA,
        dirB,
        pruneIgnored
      );
      setEntries(result);
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

  const applySyncPatch = useCallback((tasks: SyncTask[]) => {
    if (tasks.length === 0) return;
    const taskMap = new Map<string, PlannedAction>();
    for (const t of tasks) {
      if (t.action !== 'skip') {
        taskMap.set(t.relative_path, t.action);
      }
    }

    setEntries((prevEntries) => {
      const nextEntries: UnifiedEntry[] = [];
      for (const entry of prevEntries) {
        const action = taskMap.get(entry.relative_path);
        if (!action) {
          nextEntries.push(entry);
          continue;
        }

        switch (action) {
          case 'copy_a_to_b': {
            const metaA = entry.meta_a;
            nextEntries.push({
              ...entry,
              diff_kind: { type: 'Identical' },
              meta_b: metaA ? { ...metaA } : entry.meta_b,
            });
            break;
          }
          case 'copy_b_to_a': {
            const metaB = entry.meta_b;
            nextEntries.push({
              ...entry,
              diff_kind: { type: 'Identical' },
              meta_a: metaB ? { ...metaB } : entry.meta_a,
            });
            break;
          }
          case 'delete_a': {
            if (entry.diff_kind.type !== 'OnlyInA') {
              nextEntries.push({
                ...entry,
                diff_kind: { type: 'OnlyInB' },
                meta_a: null,
              });
            }
            break;
          }
          case 'delete_b': {
            if (entry.diff_kind.type !== 'OnlyInB') {
              nextEntries.push({
                ...entry,
                diff_kind: { type: 'OnlyInA' },
                meta_b: null,
              });
            }
            break;
          }
          case 'delete_both': {
            // Completely removed from both sides
            break;
          }
          default:
            nextEntries.push(entry);
            break;
        }
      }
      return nextEntries;
    });
  }, []);

  return {
    dirA,
    dirB,
    setDirA,
    setDirB,
    selectDirA,
    selectDirB,
    isLoading,
    scanProgress,
    pruneIgnored,
    setPruneIgnored,
    entries,
    setEntries,
    applySyncPatch,
    runCompare,
    cancelScan,
  };
}
