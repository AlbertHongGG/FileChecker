import React, { useState } from 'react';
import { motion } from 'framer-motion';
import { X, Columns2, Rows2, FileCode } from 'lucide-react';
import { DiffViewer } from './DiffViewer';
import { Tooltip } from './Tooltip';

interface DiffDrawerProps {
  relativePath: string;
  contentA: string;
  contentB: string;
  dirNameA: string;
  dirNameB: string;
  onClose: () => void;
}

export const DiffDrawer: React.FC<DiffDrawerProps> = ({
  relativePath,
  contentA,
  contentB,
  dirNameA,
  dirNameB,
  onClose,
}) => {
  const [splitView, setSplitView] = useState(true);

  // Guess language from extension
  const ext = relativePath.split('.').pop()?.toLowerCase() || '';
  const langMap: Record<string, string> = {
    ts: 'typescript',
    tsx: 'tsx',
    js: 'javascript',
    jsx: 'jsx',
    json: 'json',
    css: 'css',
    html: 'html',
    rs: 'rust',
    py: 'python',
    go: 'go',
    md: 'markdown',
    sql: 'sql',
    sh: 'bash',
  };
  const language = langMap[ext] || 'javascript';

  return (
    <motion.div
      initial={{ y: '100%' }}
      animate={{ y: 0 }}
      exit={{ y: '100%' }}
      transition={{ type: 'spring', damping: 30, stiffness: 350 }}
      style={{
        position: 'absolute',
        bottom: 0,
        left: 0,
        right: 0,
        height: '65%',
        background: 'var(--bg-surface-solid)',
        backdropFilter: 'blur(32px)',
        WebkitBackdropFilter: 'blur(32px)',
        borderTop: '1px solid var(--border-strong)',
        boxShadow: '0 -20px 40px rgba(0, 0, 0, 0.25)',
        display: 'flex',
        flexDirection: 'column',
        zIndex: 100,
        overflow: 'hidden',
      }}
    >
      {/* Header */}
      <div style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '8px 16px',
        borderBottom: '1px solid var(--border-subtle)',
        background: 'var(--bg-surface)',
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px', minWidth: 0 }}>
          <FileCode size={16} style={{ color: 'var(--status-info)', flexShrink: 0 }} />
          <span style={{ fontSize: '13px', fontWeight: 600, color: 'var(--text-primary)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
            {relativePath}
          </span>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: '6px', flexShrink: 0 }}>
          <Tooltip content={splitView ? "Inline View" : "Split View"} position="bottom">
            <button className="icon-btn" onClick={() => setSplitView(!splitView)}>
              {splitView ? <Rows2 size={15} /> : <Columns2 size={15} />}
            </button>
          </Tooltip>

          <Tooltip content="Close" position="bottom">
            <button className="icon-btn" onClick={onClose}>
              <X size={15} />
            </button>
          </Tooltip>
        </div>
      </div>

      {/* Body */}
      <div style={{ flex: 1, overflow: 'auto', padding: '12px' }}>
        <DiffViewer
          oldValue={contentA}
          newValue={contentB}
          splitView={splitView}
          leftTitle={`A: ${dirNameA}`}
          rightTitle={`B: ${dirNameB}`}
          language={language}
        />
      </div>
    </motion.div>
  );
};
