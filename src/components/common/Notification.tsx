import React from 'react';
import { AnimatePresence, motion } from 'framer-motion';
import { X } from 'lucide-react';
import { useNotification, NotificationMsg } from '../../contexts/NotificationContext';

const AnimatedIcon = ({ type }: { type: NotificationMsg['type'] }) => {
  const draw = {
    hidden: { pathLength: 0, opacity: 0 },
    visible: {
      pathLength: 1,
      opacity: 1,
      transition: { pathLength: { duration: 0.6, ease: "easeOut" }, opacity: { duration: 0.2 } }
    }
  };

  if (type === 'success') {
    return (
      <motion.svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="var(--status-success)" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
        <motion.path variants={draw as any} initial="hidden" animate="visible" d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
        <motion.path variants={draw as any} initial="hidden" animate="visible" d="M22 4L12 14.01l-3-3" transition={{ delay: 0.2 }} />
      </motion.svg>
    );
  }
  if (type === 'error') {
    return (
      <motion.svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="var(--status-danger)" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
        <motion.circle cx="12" cy="12" r="10" variants={draw as any} initial="hidden" animate="visible" />
        <motion.line x1="15" y1="9" x2="9" y2="15" variants={draw as any} initial="hidden" animate="visible" transition={{ delay: 0.3 }} />
        <motion.line x1="9" y1="9" x2="15" y2="15" variants={draw as any} initial="hidden" animate="visible" transition={{ delay: 0.3 }} />
      </motion.svg>
    );
  }
  if (type === 'warning') {
    return (
      <motion.svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="var(--status-warning)" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
        <motion.path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" variants={draw as any} initial="hidden" animate="visible" />
        <motion.line x1="12" y1="9" x2="12" y2="13" variants={draw as any} initial="hidden" animate="visible" transition={{ delay: 0.3 }} />
        <motion.line x1="12" y1="17" x2="12.01" y2="17" variants={draw as any} initial="hidden" animate="visible" transition={{ delay: 0.4 }} />
      </motion.svg>
    );
  }
  return (
    <motion.svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="var(--status-info)" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
      <motion.circle cx="12" cy="12" r="10" variants={draw as any} initial="hidden" animate="visible" />
      <motion.line x1="12" y1="16" x2="12" y2="12" variants={draw as any} initial="hidden" animate="visible" transition={{ delay: 0.3 }} />
      <motion.line x1="12" y1="8" x2="12.01" y2="8" variants={draw as any} initial="hidden" animate="visible" transition={{ delay: 0.4 }} />
    </motion.svg>
  );
};

const NotificationItem: React.FC<{ notification: NotificationMsg; onClose: () => void }> = ({ notification, onClose }) => {
  return (
    <motion.div
      layout
      initial={{ opacity: 0, y: 30, scale: 0.9 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      exit={{ opacity: 0, scale: 0.9, y: 20, transition: { duration: 0.2 } }}
      transition={{ type: "spring", stiffness: 400, damping: 30 }}
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: '12px',
        padding: '8px 14px',
        background: 'var(--bg-surface)',
        backdropFilter: 'blur(24px)',
        WebkitBackdropFilter: 'blur(24px)',
        borderRadius: '100px',
        border: '1px solid var(--border-default)',
        boxShadow: 'var(--shadow-glass)',
        pointerEvents: 'auto',
        maxWidth: 'none',
        width: 'fit-content'
      }}
    >
      <div style={{ flexShrink: 0, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <AnimatedIcon type={notification.type} />
      </div>

      <div style={{ flex: '1 1 auto', display: 'flex', alignItems: 'center', gap: '8px', minWidth: 0, paddingRight: '4px' }}>
        <span style={{ fontSize: '13px', fontWeight: 600, color: 'var(--text-primary)', whiteSpace: 'nowrap', flexShrink: 0 }}>
          {notification.title}
        </span>
        {notification.message && (
          <>
            <div style={{ width: '4px', height: '4px', borderRadius: '50%', background: 'var(--text-tertiary)', flexShrink: 0 }} />
            <span style={{ fontSize: '12px', color: 'var(--text-secondary)', whiteSpace: 'nowrap' }}>
              {notification.message}
            </span>
          </>
        )}
      </div>

      <motion.button 
        onClick={onClose}
        initial={{ opacity: 0.5 }}
        whileHover={{ opacity: 1, scale: 1.1, background: 'var(--bg-surface-hover)', color: 'var(--text-primary)' }}
        whileTap={{ scale: 0.9 }}
        style={{
          background: 'transparent',
          border: 'none',
          color: 'var(--text-secondary)',
          cursor: 'pointer',
          width: '22px',
          height: '22px',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          borderRadius: '50%',
          flexShrink: 0,
        }}
      >
        <X size={12} strokeWidth={2.5} />
      </motion.button>
    </motion.div>
  );
};

export const NotificationContainer: React.FC = () => {
  const { notifications, removeNotification } = useNotification();

  return (
    <div style={{
      position: 'fixed',
      bottom: '24px',
      left: '50%',
      transform: 'translateX(-50%)',
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      gap: '8px',
      zIndex: 9999,
      pointerEvents: 'none',
    }}>
      <AnimatePresence mode="popLayout">
        {notifications.map(n => (
          <NotificationItem 
            key={n.id} 
            notification={n} 
            onClose={() => removeNotification(n.id)} 
          />
        ))}
      </AnimatePresence>
    </div>
  );
};
