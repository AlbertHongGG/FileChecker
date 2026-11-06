import React, { useState, useRef, useEffect } from 'react';
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
  delay = 250 
}) => {
  const [isVisible, setIsVisible] = useState(false);
  const [rect, setRect] = useState<DOMRect | null>(null);
  const targetRef = useRef<HTMLDivElement>(null);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  const handleMouseEnter = () => {
    if (targetRef.current) {
      setRect(targetRef.current.getBoundingClientRect());
    }
    timeoutRef.current = setTimeout(() => setIsVisible(true), delay);
  };

  const handleMouseLeave = () => {
    if (timeoutRef.current) clearTimeout(timeoutRef.current);
    setIsVisible(false);
  };

  useEffect(() => {
    return () => {
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
    };
  }, []);

  const getPositionStyles = () => {
    if (!rect) return {};
    
    const styles: React.CSSProperties = {
      position: 'fixed',
      zIndex: 9999,
      pointerEvents: 'none',
    };

    switch (position) {
      case 'top':
        styles.top = rect.top - 6;
        styles.left = rect.left + rect.width / 2;
        styles.transform = 'translate(-50%, -100%)';
        break;
      case 'bottom':
        styles.top = rect.bottom + 6;
        styles.left = rect.left + rect.width / 2;
        styles.transform = 'translate(-50%, 0)';
        break;
      case 'bottom-end':
        styles.top = rect.bottom + 6;
        styles.left = rect.right;
        styles.transform = 'translate(-100%, 0)';
        break;
      case 'left':
        styles.top = rect.top + rect.height / 2;
        styles.left = rect.left - 6;
        styles.transform = 'translate(-100%, -50%)';
        break;
      case 'right':
        styles.top = rect.top + rect.height / 2;
        styles.left = rect.right + 6;
        styles.transform = 'translate(0, -50%)';
        break;
    }
    return styles;
  };

  const animationVariants = {
    top: { initial: { opacity: 0, y: 4 }, animate: { opacity: 1, y: 0 }, exit: { opacity: 0, y: 4 } },
    bottom: { initial: { opacity: 0, y: -4 }, animate: { opacity: 1, y: 0 }, exit: { opacity: 0, y: -4 } },
    'bottom-end': { initial: { opacity: 0, y: -4 }, animate: { opacity: 1, y: 0 }, exit: { opacity: 0, y: -4 } },
    left: { initial: { opacity: 0, x: 4 }, animate: { opacity: 1, x: 0 }, exit: { opacity: 0, x: 4 } },
    right: { initial: { opacity: 0, x: -4 }, animate: { opacity: 1, x: 0 }, exit: { opacity: 0, x: -4 } },
  };

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
            initial={animationVariants[position].initial}
            animate={animationVariants[position].animate}
            exit={animationVariants[position].exit}
            transition={{ duration: 0.15, ease: 'easeOut' }}
            style={{
              ...getPositionStyles(),
              backgroundColor: 'var(--text-primary)',
              color: 'var(--bg-base)',
              padding: '4px 10px',
              borderRadius: 'var(--radius-sm)',
              fontSize: '11px',
              fontWeight: 500,
              whiteSpace: 'nowrap',
              boxShadow: 'var(--shadow-md)',
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
