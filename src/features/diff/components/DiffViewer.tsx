import React from 'react';
import ReactDiffViewer from 'react-diff-viewer-continued';
import { useTheme } from '../../../contexts/ThemeContext';
import Prism from 'prismjs';
import 'prismjs/components/prism-javascript';
import 'prismjs/components/prism-typescript';
import 'prismjs/components/prism-jsx';
import 'prismjs/components/prism-tsx';
import 'prismjs/components/prism-css';
import 'prismjs/components/prism-json';
import 'prismjs/components/prism-bash';
import 'prismjs/components/prism-markdown';
import 'prismjs/components/prism-rust';
import 'prismjs/components/prism-python';
import 'prismjs/components/prism-go';
import 'prismjs/components/prism-sql';
import 'prismjs/themes/prism-tomorrow.css';

interface DiffViewerProps {
  oldValue: string;
  newValue: string;
  splitView?: boolean;
  leftTitle?: string;
  rightTitle?: string;
  language?: string;
}

export const DiffViewer: React.FC<DiffViewerProps> = ({
  oldValue,
  newValue,
  splitView = true,
  leftTitle,
  rightTitle,
  language = 'typescript',
}) => {
  const { theme } = useTheme();

  const highlightSyntax = (str: string) => {
    if (!str) return <span></span>;
    try {
      const lang = Prism.languages[language] || Prism.languages.javascript;
      return (
        <span
          dangerouslySetInnerHTML={{
            __html: Prism.highlight(str, lang, language),
          }}
        />
      );
    } catch {
      return <span>{str}</span>;
    }
  };

  return (
    <div
      className="diff-viewer-container"
      style={{
        borderRadius: 'var(--radius-md)',
        overflow: 'hidden',
        border: '1px solid var(--border-default)',
      }}
    >
      <ReactDiffViewer
        oldValue={oldValue}
        newValue={newValue}
        splitView={splitView}
        useDarkTheme={theme === 'dark'}
        leftTitle={leftTitle}
        rightTitle={rightTitle}
        renderContent={highlightSyntax}
        styles={{
          variables: {
            dark: {
              diffViewerBackground: 'var(--bg-base)',
              diffViewerColor: 'var(--text-primary)',
              addedBackground: 'transparent',
              addedColor: 'var(--text-primary)',
              removedBackground: 'transparent',
              removedColor: 'var(--text-primary)',
              wordAddedBackground: 'rgba(52, 211, 153, 0.25)',
              wordRemovedBackground: 'rgba(248, 113, 113, 0.25)',
              addedGutterBackground: 'rgba(52, 211, 153, 0.15)',
              removedGutterBackground: 'rgba(248, 113, 113, 0.15)',
              gutterBackground: 'var(--bg-surface)',
              gutterBackgroundDark: 'var(--bg-surface-hover)',
              highlightBackground: 'var(--bg-surface-solid)',
              highlightGutterBackground: 'var(--bg-surface-solid)',
              codeFoldGutterBackground: 'var(--bg-surface)',
              codeFoldBackground: 'var(--bg-surface)',
              emptyLineBackground: 'var(--bg-base)',
              gutterColor: 'var(--text-tertiary)',
              addedGutterColor: 'var(--status-success)',
              removedGutterColor: 'var(--status-danger)',
              codeFoldContentColor: 'var(--text-secondary)',
              diffViewerTitleBackground: 'var(--bg-surface)',
              diffViewerTitleColor: 'var(--text-primary)',
              diffViewerTitleBorderColor: 'var(--border-subtle)',
            },
            light: {
              diffViewerBackground: 'var(--bg-base)',
              diffViewerColor: 'var(--text-primary)',
              addedBackground: 'transparent',
              addedColor: 'var(--text-primary)',
              removedBackground: 'transparent',
              removedColor: 'var(--text-primary)',
              wordAddedBackground: 'rgba(16, 185, 129, 0.25)',
              wordRemovedBackground: 'rgba(239, 68, 68, 0.25)',
              addedGutterBackground: 'rgba(16, 185, 129, 0.15)',
              removedGutterBackground: 'rgba(239, 68, 68, 0.15)',
              gutterBackground: 'var(--bg-surface)',
              gutterBackgroundDark: 'var(--bg-surface-hover)',
              highlightBackground: 'var(--bg-surface-solid)',
              highlightGutterBackground: 'var(--bg-surface-solid)',
              codeFoldGutterBackground: 'var(--bg-surface)',
              codeFoldBackground: 'var(--bg-surface)',
              emptyLineBackground: 'var(--bg-base)',
              gutterColor: 'var(--text-tertiary)',
              addedGutterColor: 'var(--status-success)',
              removedGutterColor: 'var(--status-danger)',
              codeFoldContentColor: 'var(--text-secondary)',
              diffViewerTitleBackground: 'var(--bg-surface)',
              diffViewerTitleColor: 'var(--text-primary)',
              diffViewerTitleBorderColor: 'var(--border-subtle)',
            },
          },
          line: {
            padding: '3px 8px',
            lineHeight: 1.5,
            fontSize: '12px',
            fontFamily: 'Consolas, Monaco, "Courier New", monospace',
          },
          contentText: {
            lineHeight: 1.5,
            fontFamily: 'Consolas, Monaco, "Courier New", monospace',
            wordBreak: 'keep-all',
            whiteSpace: 'pre',
          },
          gutter: {
            padding: '3px 8px',
            fontSize: '11px',
          },
          titleBlock: {
            padding: '10px 14px',
            fontSize: '12px',
          },
        }}
      />
    </div>
  );
};
