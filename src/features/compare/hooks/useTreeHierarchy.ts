import { useState, useMemo, useEffect, useCallback } from 'react';
import { UnifiedEntry, PlannedAction, FlattenedRow } from '../../../types';
import { buildTree, flattenTree } from '../utils/treeTransformer';

export function useTreeHierarchy(
  entries: UnifiedEntry[],
  actionMap: Map<string, PlannedAction>
) {
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(new Set());
  const [onlyDiffs, setOnlyDiffs] = useState(true);

  // Auto-expand paths with differences when entries update
  useEffect(() => {
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
  }, [entries]);

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
    () => flattenTree(tree, expandedPaths, onlyDiffs, actionMap),
    [tree, expandedPaths, onlyDiffs, actionMap]
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
