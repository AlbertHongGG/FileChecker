import { useState, useEffect, useCallback } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { TauriClient } from '../../../services/tauriClient';
import { EventListenerService } from '../../../services/eventListener';
import { UnifiedEntry, ScanProgressPayload } from '../../../types';
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
      count_a: 0,
      count_b: 0,
      is_hdd_a: false,
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
    runCompare,
    cancelScan,
  };
}
