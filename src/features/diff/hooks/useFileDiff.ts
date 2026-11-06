import { useState, useCallback } from 'react';
import { TauriClient } from '../../../services/tauriClient';
import { useNotification } from '../../../contexts/NotificationContext';

export interface DiffTarget {
  path: string;
  contentA: string;
  contentB: string;
}

export function useFileDiff(dirA: string | null, dirB: string | null) {
  const [diffTarget, setDiffTarget] = useState<DiffTarget | null>(null);
  const [isLoadingDiff, setIsLoadingDiff] = useState(false);
  const { addNotification } = useNotification();

  const openDiff = useCallback(
    async (relativePath: string) => {
      if (!dirA || !dirB) return;
      setIsLoadingDiff(true);
      try {
        const [contentA, contentB] = await TauriClient.getFileContentPair(
          dirA,
          dirB,
          relativePath
        );
        setDiffTarget({
          path: relativePath,
          contentA,
          contentB,
        });
      } catch (err) {
        console.error('Failed to load diff content:', err);
        addNotification('error', 'Diff Error', 'Failed to read file contents');
      } finally {
        setIsLoadingDiff(false);
      }
    },
    [dirA, dirB, addNotification]
  );

  const closeDiff = useCallback(() => {
    setDiffTarget(null);
  }, []);

  return {
    diffTarget,
    isLoadingDiff,
    openDiff,
    closeDiff,
  };
}
