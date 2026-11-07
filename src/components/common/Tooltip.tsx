import React, { useState, useRef, useEffect, useLayoutEffect } from 'react';
import { createPortal } from 'react-dom';
import { AnimatePresence, motion } from 'framer-motion';

export interface TooltipProps {
  content: React.ReactNode;
  children: React.ReactElement;
  position?: 'top' | 'bottom' | 'left' | 'right' | 'bottom-end';
  delay?: number;
}

export const Tooltip: React.FC<TooltipProps> = ({ 
  content, 
  children, 
  position = 'top',
  delay = 200 
}) => {
  const [isVisible, setIsVisible] = useState(false);
  const [rect, setRect] = useState<DOMRect | null>(null);
  const [coords, setCoords] = useState<{ top: number; left: number } | null>(null);
  const targetRef = useRef<HTMLDivElement>(null);
  const tooltipRef = useRef<HTMLDivElement>(null);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  const handleMouseEnter = () => {
    if (targetRef.current) {
      setRect(targetRef.current.getBoundingClientRect());
    }
    timeoutRef.current = setTimeout(() => {
      setIsVisible(true);
    }, delay);
  };

  const handleMouseLeave = () => {
    if (timeoutRef.current) clearTimeout(timeoutRef.current);
    setIsVisible(false);
    setCoords(null);
  };

  useEffect(() => {
    return () => {
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
    };
  }, []);

  // Viewport Collision & Clamping Engine
  useLayoutEffect(() => {
    if (!isVisible || !rect || !tooltipRef.current) return;

    const tooltipEl = tooltipRef.current;
    const tooltipWidth = tooltipEl.offsetWidth;
    const tooltipHeight = tooltipEl.offsetHeight;
    const viewportWidth = window.innerWidth;
    const viewportHeight = window.innerHeight;
    const SAFE_PADDING = 10; // Keep at least 10px away from window edges

    let top = 0;
    let left = 0;

    switch (position) {
      case 'top': {
        top = rect.top - tooltipHeight - 6;
        left = rect.left + rect.width / 2 - tooltipWidth / 2;
        break;
      }
      case 'bottom':
      case 'bottom-end': {
        top = rect.bottom + 6;
        left = rect.left + rect.width / 2 - tooltipWidth / 2;
        break;
      }
      case 'left': {
        top = rect.top + rect.height / 2 - tooltipHeight / 2;
        left = rect.left - tooltipWidth - 6;
        break;
      }
      case 'right': {
        top = rect.top + rect.height / 2 - tooltipHeight / 2;
        left = rect.right + 6;
        break;
      }
    }

    // Horizontal boundary clamping: keep within [SAFE_PADDING, viewportWidth - tooltipWidth - SAFE_PADDING]
    if (left + tooltipWidth > viewportWidth - SAFE_PADDING) {
      left = viewportWidth - tooltipWidth - SAFE_PADDING;
    }
    if (left < SAFE_PADDING) {
      left = SAFE_PADDING;
    }

    // Vertical boundary flip if out of viewport
    if (top < SAFE_PADDING && position === 'top') {
      top = rect.bottom + 6;
    } else if (
      top + tooltipHeight > viewportHeight - SAFE_PADDING &&
      (position === 'bottom' || position === 'bottom-end')
    ) {
      top = rect.top - tooltipHeight - 6;
    }

    setCoords({ top, left });
  }, [isVisible, rect, position, content]);

  return (
    <>
      <div 
        ref={targetRef}
        style={{ display: 'inline-flex' }}
        onMouseEnter={handleMouseEnter}
        onMouseLeave={handleMouseLeave}
      >
        {children}
      </div>
      {isVisible && rect && createPortal(
        <AnimatePresence>
          <motion.div
            ref={tooltipRef}
            initial={{ opacity: 0, scale: 0.96 }}
            animate={{ opacity: coords ? 1 : 0, scale: coords ? 1 : 0.96 }}
            exit={{ opacity: 0, scale: 0.96 }}
            transition={{ duration: 0.12, ease: 'easeOut' }}
            style={{
              position: 'fixed',
              top: coords ? `${coords.top}px` : '-9999px',
              left: coords ? `${coords.left}px` : '-9999px',
              zIndex: 9999,
              pointerEvents: 'none',
              backgroundColor: 'var(--text-primary)',
              color: 'var(--bg-base)',
              padding: '4px 10px',
              borderRadius: 'var(--radius-sm)',
              fontSize: '11px',
              fontWeight: 500,
              whiteSpace: 'nowrap',
              boxShadow: 'var(--shadow-md)',
              visibility: coords ? 'visible' : 'hidden',
            }}
          >
            {content}
          </motion.div>
        </AnimatePresence>,
        document.body
      )}
    </>
  );
};
