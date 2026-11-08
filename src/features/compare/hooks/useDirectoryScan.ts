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
  const [scanVersion, setScanVersion] = useState(0);

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
    setEntries([]);
    setIsLoading(true);
    setScanProgress({
      side_a: {
        engine: 'Preparing...',
        is_hdd: false,
        percent: 0,
        stage: { kind: 'Initializing' },
      },
      side_b: {
        engine: 'Preparing...',
        is_hdd: false,
        percent: 0,
        stage: { kind: 'Initializing' },
      },
    });

    try {
      const result = await TauriClient.compareDirectories(
        dirA,
        dirB,
        pruneIgnored
      );
      setEntries(result);
      setScanVersion((v) => v + 1);
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
    const copiedToBParents = new Set<string>();
    const copiedToAParents = new Set<string>();

    for (const t of tasks) {
      if (t.action !== 'skip') {
        taskMap.set(t.relative_path, t.action);

        if (t.action === 'copy_a_to_b') {
          const parts = t.relative_path.split('/');
          let cur = '';
          for (let i = 0; i < parts.length - 1; i++) {
            cur = cur ? `${cur}/${parts[i]}` : parts[i];
            copiedToBParents.add(cur);
          }
        } else if (t.action === 'copy_b_to_a') {
          const parts = t.relative_path.split('/');
          let cur = '';
          for (let i = 0; i < parts.length - 1; i++) {
            cur = cur ? `${cur}/${parts[i]}` : parts[i];
            copiedToAParents.add(cur);
          }
        }
      }
    }

    setEntries((prevEntries) => {
      const nextEntries: UnifiedEntry[] = [];
      for (const entry of prevEntries) {
        const action = taskMap.get(entry.relative_path);
        if (!action) {
          // If this is a directory and children were copied into it, update directory presence
          if (entry.is_dir) {
            let updatedMetaA = entry.meta_a;
            let updatedMetaB = entry.meta_b;
            let changed = false;

            if (copiedToAParents.has(entry.relative_path) && !updatedMetaA) {
              updatedMetaA = { size: 0, modified_timestamp: 0, is_dir: true };
              changed = true;
            }
            if (copiedToBParents.has(entry.relative_path) && !updatedMetaB) {
              updatedMetaB = { size: 0, modified_timestamp: 0, is_dir: true };
              changed = true;
            }

            if (changed) {
              nextEntries.push({
                ...entry,
                meta_a: updatedMetaA,
                meta_b: updatedMetaB,
              });
              continue;
            }
          }

          nextEntries.push(entry);
          continue;
        }

        switch (action) {
          case 'copy_a_to_b': {
            const metaA = entry.meta_a;
            nextEntries.push({
              ...entry,
              diff_kind: { type: 'Identical' },
              meta_b: metaA ? { ...metaA } : (entry.is_dir ? { size: 0, modified_timestamp: 0, is_dir: true } : entry.meta_b),
            });
            break;
          }
          case 'copy_b_to_a': {
            const metaB = entry.meta_b;
            nextEntries.push({
              ...entry,
              diff_kind: { type: 'Identical' },
              meta_a: metaB ? { ...metaB } : (entry.is_dir ? { size: 0, modified_timestamp: 0, is_dir: true } : entry.meta_a),
            });
            break;
          }
          case 'delete_a': {
            // Deleted from Side A
            if (entry.meta_b) {
              // Still exists on Side B
              nextEntries.push({
                ...entry,
                diff_kind: { type: 'OnlyInB' },
                meta_a: null,
              });
            }
            // If neither side has it, it is dropped from nextEntries
            break;
          }
          case 'delete_b': {
            // Deleted from Side B
            if (entry.meta_a) {
              // Still exists on Side A
              nextEntries.push({
                ...entry,
                diff_kind: { type: 'OnlyInA' },
                meta_b: null,
              });
            }
            // If neither side has it, it is dropped from nextEntries
            break;
          }
          case 'delete_both': {
            // Completely removed from both sides, omitted from nextEntries
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
    scanVersion,
    applySyncPatch,
    runCompare,
    cancelScan,
  };
}
