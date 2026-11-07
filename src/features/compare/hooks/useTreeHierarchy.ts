import { useState, useMemo, useEffect, useCallback } from 'react';
import { UnifiedEntry, FlattenedRow } from '../../../types';
import { buildTree, flattenTree } from '../utils/treeTransformer';

export function useTreeHierarchy(
  entries: UnifiedEntry[],
  scanVersion: number
) {
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(new Set());
  const [onlyDiffs, setOnlyDiffs] = useState(true);

  // Auto-expand paths with differences ONLY when a fresh scan completes (scanVersion changes)
  useEffect(() => {
    if (entries.length === 0) {
      setExpandedPaths(new Set());
      return;
    }
    const autoExpand = new Set<string>();
    for (const e of entries) {
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
  }, [scanVersion]);

  const toggleExpand = useCallback((path: string) => {
    setExpandedPaths((prev) => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  }, []);

  const tree = useMemo(() => buildTree(entries), [entries]);

  const rows: FlattenedRow[] = useMemo(
    () => flattenTree(tree, expandedPaths, onlyDiffs),
    [tree, expandedPaths, onlyDiffs]
  );

  return {
    tree,
    rows,
    expandedPaths,
    toggleExpand,
    onlyDiffs,
    setOnlyDiffs,
  };
}
