import { useState, useCallback } from 'react';
import { TauriClient, FileDiffResult } from '../../../services/tauriClient';
import { useNotification } from '../../../contexts/NotificationContext';
import { TreeNode } from '../../../types';
import { collectModifiableFiles } from '../../compare/utils/subtreeQuery';

export interface DiffSession {
  mode: 'file' | 'directory';
  targetPath: string;
  files: string[];
  activeIndex: number;
  data: FileDiffResult | null;
}

export function useFileDiff(dirA: string | null, dirB: string | null) {
  const [session, setSession] = useState<DiffSession | null>(null);
  const [isLoadingDiff, setIsLoadingDiff] = useState(false);
  const { addNotification } = useNotification();

  const loadFileDiff = useCallback(
    async (filePath: string, currentSession: DiffSession) => {
      if (!dirA || !dirB) return;
      setIsLoadingDiff(true);
      try {
        const result = await TauriClient.getFileContentPair(dirA, dirB, filePath);
        setSession({
          ...currentSession,
          data: result,
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

  const openDiff = useCallback(
    async (node: TreeNode) => {
      if (!dirA || !dirB) return;

      if (node.is_dir) {
        const modifiable = collectModifiableFiles(node);
        if (modifiable.length === 0) return;

        const newSession: DiffSession = {
          mode: 'directory',
          targetPath: node.relative_path,
          files: modifiable,
          activeIndex: 0,
          data: null,
        };
        setSession(newSession);
        await loadFileDiff(modifiable[0], newSession);
      } else {
        const newSession: DiffSession = {
          mode: 'file',
          targetPath: node.relative_path,
          files: [node.relative_path],
          activeIndex: 0,
          data: null,
        };
        setSession(newSession);
        await loadFileDiff(node.relative_path, newSession);
      }
    },
    [dirA, dirB, loadFileDiff]
  );

  const selectDiffFile = useCallback(
    async (index: number) => {
      if (!session || index < 0 || index >= session.files.length) return;
      const targetFile = session.files[index];
      const updatedSession = { ...session, activeIndex: index };
      setSession(updatedSession);
      await loadFileDiff(targetFile, updatedSession);
    },
    [session, loadFileDiff]
  );

  const closeDiff = useCallback(() => {
    setSession(null);
  }, []);

  return {
    diffSession: session,
    isLoadingDiff,
    openDiff,
    selectDiffFile,
    closeDiff,
  };
}
