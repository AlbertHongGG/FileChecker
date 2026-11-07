import React, { useState } from 'react';
import { motion } from 'framer-motion';
import { X, Columns2, Rows2, FileCode, Folder, AlertCircle, Loader2 } from 'lucide-react';
import { DiffViewer } from './DiffViewer';
import { Tooltip } from '../../../components/common/Tooltip';
import { DiffSession } from '../hooks/useFileDiff';
import { formatFileSize } from '../../compare/utils/formatters';

interface DiffDrawerProps {
  session: DiffSession;
  isLoading: boolean;
  dirNameA: string;
  dirNameB: string;
  onSelectFile: (index: number) => void;
  onClose: () => void;
}

export const DiffDrawer: React.FC<DiffDrawerProps> = ({
  session,
  isLoading,
  dirNameA,
  dirNameB,
  onSelectFile,
  onClose,
}) => {
  const [splitView, setSplitView] = useState(true);

  const activeFilePath = session.files[session.activeIndex] || session.targetPath;
  const ext = activeFilePath.split('.').pop()?.toLowerCase() || '';
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

  const isMultiFile = session.files.length > 1;
  const data = session.data;

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
        height: '70%',
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
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '8px 16px',
          borderBottom: '1px solid var(--border-subtle)',
          background: 'var(--bg-surface)',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px', minWidth: 0 }}>
          {session.mode === 'directory' ? (
            <Folder size={16} style={{ color: 'var(--status-info)', flexShrink: 0 }} />
          ) : (
            <FileCode size={16} style={{ color: 'var(--status-info)', flexShrink: 0 }} />
          )}
          <span
            style={{
              fontSize: '13px',
              fontWeight: 600,
              color: 'var(--text-primary)',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
          >
            {activeFilePath}
          </span>
          {session.mode === 'directory' && (
            <span
              style={{
                fontSize: '11px',
                padding: '1px 6px',
                borderRadius: '10px',
                background: 'var(--bg-base)',
                color: 'var(--text-tertiary)',
              }}
            >
              {session.activeIndex + 1} / {session.files.length} modified
            </span>
          )}
          {isLoading && <Loader2 size={13} className="animate-spin" style={{ color: 'var(--status-info)' }} />}
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: '6px', flexShrink: 0 }}>
          <Tooltip content={splitView ? 'Inline View' : 'Split View'} position="bottom">
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

      {/* Main Diff Content Container */}
      <div style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
        {/* Left Multi-File Sidebar when viewing Directory Diff */}
        {isMultiFile && (
          <div
            style={{
              width: '240px',
              borderRight: '1px solid var(--border-subtle)',
              background: 'var(--bg-surface)',
              overflowY: 'auto',
              display: 'flex',
              flexDirection: 'column',
              flexShrink: 0,
            }}
          >
            <div
              style={{
                padding: '6px 12px',
                fontSize: '11px',
                fontWeight: 600,
                color: 'var(--text-tertiary)',
                borderBottom: '1px solid var(--border-subtle)',
              }}
            >
              Modified Files in Folder
            </div>
            {session.files.map((file, idx) => {
              const fileName = file.split('/').pop() || file;
              const isSelected = idx === session.activeIndex;
              return (
                <div
                  key={file}
                  onClick={() => onSelectFile(idx)}
                  style={{
                    padding: '6px 12px',
                    fontSize: '12px',
                    cursor: 'pointer',
                    display: 'flex',
                    alignItems: 'center',
                    gap: '6px',
                    background: isSelected ? 'var(--bg-surface-active)' : 'transparent',
                    color: isSelected ? 'var(--status-info)' : 'var(--text-secondary)',
                    borderLeft: isSelected ? '3px solid var(--status-info)' : '3px solid transparent',
                  }}
                  onMouseEnter={(e) => {
                    if (!isSelected) e.currentTarget.style.backgroundColor = 'var(--bg-surface-hover)';
                  }}
                  onMouseLeave={(e) => {
                    if (!isSelected) e.currentTarget.style.backgroundColor = 'transparent';
                  }}
                >
                  <FileCode size={13} style={{ flexShrink: 0 }} />
                  <span
                    title={file}
                    style={{
                      overflow: 'hidden',
                      textOverflow: 'ellipsis',
                      whiteSpace: 'nowrap',
                      flex: 1,
                    }}
                  >
                    {fileName}
                  </span>
                </div>
              );
            })}
          </div>
        )}

        {/* Diff Content / Fallback Viewer */}
        <div style={{ flex: 1, overflow: 'auto', padding: '12px', position: 'relative' }}>
          {isLoading && !data ? (
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                height: '100%',
                color: 'var(--text-tertiary)',
                gap: '8px',
              }}
            >
              <Loader2 size={16} className="animate-spin" />
              <span>Loading differences...</span>
            </div>
          ) : data?.is_binary ? (
            <div
              style={{
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                justifyContent: 'center',
                height: '100%',
                gap: '12px',
                color: 'var(--text-secondary)',
              }}
            >
              <AlertCircle size={32} style={{ color: 'var(--status-warning)' }} />
              <div style={{ fontSize: '14px', fontWeight: 600 }}>Binary File Comparison</div>
              <div style={{ fontSize: '12px', color: 'var(--text-tertiary)' }}>
                Visual text diff is disabled for binary files.
              </div>
              <div
                style={{
                  display: 'flex',
                  gap: '24px',
                  marginTop: '8px',
                  padding: '12px 24px',
                  background: 'var(--bg-surface)',
                  borderRadius: '8px',
                  border: '1px solid var(--border-subtle)',
                }}
              >
                <div>
                  <strong>{dirNameA}:</strong> {formatFileSize(data.size_a)}
                </div>
                <div>
                  <strong>{dirNameB}:</strong> {formatFileSize(data.size_b)}
                </div>
              </div>
            </div>
          ) : data?.is_too_large ? (
            <div
              style={{
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                justifyContent: 'center',
                height: '100%',
                gap: '12px',
                color: 'var(--text-secondary)',
              }}
            >
              <AlertCircle size={32} style={{ color: 'var(--status-warning)' }} />
              <div style={{ fontSize: '14px', fontWeight: 600 }}>File Too Large to Diff</div>
              <div style={{ fontSize: '12px', color: 'var(--text-tertiary)' }}>
                File size exceeds the 2MB in-memory preview limit.
              </div>
              <div
                style={{
                  display: 'flex',
                  gap: '24px',
                  marginTop: '8px',
                  padding: '12px 24px',
                  background: 'var(--bg-surface)',
                  borderRadius: '8px',
                  border: '1px solid var(--border-subtle)',
                }}
              >
                <div>
                  <strong>{dirNameA}:</strong> {formatFileSize(data.size_a)}
                </div>
                <div>
                  <strong>{dirNameB}:</strong> {formatFileSize(data.size_b)}
                </div>
              </div>
            </div>
          ) : data ? (
            <DiffViewer
              oldValue={data.content_a}
              newValue={data.content_b}
              splitView={splitView}
              leftTitle={`A: ${dirNameA}`}
              rightTitle={`B: ${dirNameB}`}
              language={language}
            />
          ) : null}
        </div>
      </div>
    </motion.div>
  );
};
