import { useState, useEffect, useCallback } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { TauriClient } from '../../../services/tauriClient';
import { EventListenerService } from '../../../services/eventListener';
import { UnifiedEntry, ScanProgressPayload, SyncTask } from '../../../types';
import { useNotification } from '../../../contexts/NotificationContext';
import { applySyncTasksToEntries } from '../utils/treePatcher';

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
    setEntries((prevEntries) => applySyncTasksToEntries(prevEntries, tasks));
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
