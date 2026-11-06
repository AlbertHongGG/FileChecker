import { useState, useRef, useEffect, useMemo, useCallback } from 'react';
import { FlattenedRow } from '../../../types';

export const ROW_HEIGHT = 28;
export const BUFFER_SIZE = 12;

export function useVirtualScroll(rows: FlattenedRow[]) {
  const scrollContainerRef = useRef<HTMLDivElement>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewportHeight, setViewportHeight] = useState(600);

  useEffect(() => {
    const updateHeight = () => {
      if (scrollContainerRef.current) {
        setViewportHeight(scrollContainerRef.current.clientHeight);
      }
    };
    updateHeight();
    window.addEventListener('resize', updateHeight);
    return () => window.removeEventListener('resize', updateHeight);
  }, []);

  const handleScroll = useCallback((e: React.UIEvent<HTMLDivElement>) => {
    setScrollTop(e.currentTarget.scrollTop);
  }, []);

  const { visibleRows, startIndex, totalHeight, paddingTop } = useMemo(() => {
    const count = rows.length;
    const total = count * ROW_HEIGHT;
    const start = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - BUFFER_SIZE);
    const visibleCount = Math.ceil(viewportHeight / ROW_HEIGHT) + 2 * BUFFER_SIZE;
    const end = Math.min(count, start + visibleCount);

    return {
      visibleRows: rows.slice(start, end),
      startIndex: start,
      totalHeight: total,
      paddingTop: start * ROW_HEIGHT,
    };
  }, [rows, scrollTop, viewportHeight]);

  return {
    scrollContainerRef,
    handleScroll,
    visibleRows,
    startIndex,
    totalHeight,
    paddingTop,
  };
}
