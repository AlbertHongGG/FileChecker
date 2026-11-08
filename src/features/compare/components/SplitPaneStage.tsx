import React from 'react';
import {
  FolderPlus,
  FolderCheck,
  HardDrive,
  Cpu,
  Zap,
} from 'lucide-react';
import { ScanProgressPayload, SideScanProgress, ScanStage } from '../../../types';

function formatStageDetails(stage: ScanStage): { label: string; stats: string } {
  switch (stage.kind) {
    case 'Initializing':
      return { label: 'Preparing...', stats: 'Initializing' };

    case 'ReadingMftBlocks':
      return {
        label: 'Reading MFT Blocks',
        stats: `${stage.data.completed_blocks}/${stage.data.total_blocks} blocks`,
      };

    case 'BuildingIndex':
      return {
        label: 'Building Index',
        stats: `${stage.data.records_parsed.toLocaleString()} records`,
      };

    case 'ExtractingSubtree':
      return {
        label: 'Extracting Subtree',
        stats: `${stage.data.files_indexed.toLocaleString()} files`,
      };

    case 'WalkingDirectories':
      return {
        label: stage.data.current_dir ? `[${stage.data.current_dir}]` : 'Scanning...',
        stats: `${stage.data.completed_dirs}/${stage.data.total_dirs} (${stage.data.files_scanned.toLocaleString()} files)`,
      };

    case 'Completed':
      return {
        label: 'Ready',
        stats: `${stage.data.total_files.toLocaleString()} files`,
      };

    case 'Cancelled':
      return { label: 'Cancelled', stats: 'Aborted' };

    case 'Failed':
      return { label: 'Error', stats: stage.data.error || 'Failed' };
  }
}

interface PaneStatusCardProps {
  side: 'A' | 'B';
  dirPath: string | null;
  isLoading: boolean;
  progress: SideScanProgress | null;
  onSelect: () => void;
}

export const PaneStatusCard: React.FC<PaneStatusCardProps> = ({
  side,
  dirPath,
  isLoading,
  progress,
  onSelect,
}) => {
  const label = `Directory ${side}`;

  // State 1: Empty - prompt user to select directory
  if (!dirPath) {
    return (
      <div
        onClick={onSelect}
        style={{
          flex: 1,
          margin: '16px',
          borderRadius: '12px',
          border: '1.5px dashed var(--border-strong)',
          background: 'var(--bg-surface)',
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          cursor: 'pointer',
          gap: '10px',
          transition: 'all 0.2s ease',
          userSelect: 'none',
        }}
        onMouseEnter={(e) => {
          e.currentTarget.style.borderColor = 'var(--status-info)';
          e.currentTarget.style.backgroundColor = 'var(--bg-surface-hover)';
        }}
        onMouseLeave={(e) => {
          e.currentTarget.style.borderColor = 'var(--border-strong)';
          e.currentTarget.style.backgroundColor = 'var(--bg-surface)';
        }}
      >
        <div
          style={{
            width: '48px',
            height: '48px',
            borderRadius: '50%',
            background: 'var(--bg-base)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            color: 'var(--text-secondary)',
          }}
        >
          <FolderPlus size={24} />
        </div>
        <div style={{ textAlign: 'center' }}>
          <div style={{ fontSize: '13px', fontWeight: 600, color: 'var(--text-primary)' }}>
            Select {label}
          </div>
          <div style={{ fontSize: '11px', color: 'var(--text-tertiary)', marginTop: '2px' }}>
            Click to choose a folder to compare
          </div>
        </div>
      </div>
    );
  }

  // State 2: Scanning in progress
  if (isLoading) {
    const percent = progress?.percent ?? 0;
    const stageDetails = progress ? formatStageDetails(progress.stage) : { label: 'Scanning...', stats: '0%' };
    const engineText = progress?.engine || (progress?.is_hdd ? 'HDD (Win32)' : 'SSD (Win32)');
    const isMft = engineText.includes('MFT');

    return (
      <div
        style={{
          flex: 1,
          margin: '16px',
          borderRadius: '12px',
          border: '1px solid var(--border-subtle)',
          background: 'var(--bg-surface)',
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          padding: '24px',
          boxSizing: 'border-box',
          gap: '16px',
        }}
      >
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            width: '100%',
            maxWidth: '320px',
            gap: '14px',
          }}
        >
          {/* Header with media badge & percentage */}
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
              <span
                style={{
                  fontSize: '11px',
                  fontWeight: 600,
                  padding: '2px 6px',
                  borderRadius: '4px',
                  background: isMft
                    ? 'var(--status-success-light)'
                    : 'var(--status-info-light)',
                  color: isMft
                    ? 'var(--status-success)'
                    : 'var(--status-info)',
                  display: 'flex',
                  alignItems: 'center',
                  gap: '4px',
                }}
              >
                {isMft ? (
                  <Zap size={11} />
                ) : progress?.is_hdd ? (
                  <HardDrive size={11} />
                ) : (
                  <Cpu size={11} />
                )}
                {engineText}
              </span>
            </div>
            <span style={{ fontSize: '13px', fontWeight: 700, color: 'var(--text-primary)', fontFamily: 'monospace' }}>
              {percent}%
            </span>
          </div>

          {/* Progress Bar */}
          <div
            style={{
              width: '100%',
              height: '6px',
              borderRadius: '3px',
              background: 'var(--border-subtle)',
              overflow: 'hidden',
            }}
          >
            <div
              style={{
                width: `${percent}%`,
                height: '100%',
                background: 'var(--status-info)',
                transition: 'width 0.15s ease',
                borderRadius: '3px',
              }}
            />
          </div>

          {/* Current Anchor & Count Stats */}
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              fontSize: '11px',
              color: 'var(--text-tertiary)',
            }}
          >
            <span
              style={{
                maxWidth: '180px',
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
                color: 'var(--text-secondary)',
              }}
              title={stageDetails.label}
            >
              {stageDetails.label}
            </span>
            <span style={{ fontFamily: 'monospace' }}>
              {stageDetails.stats}
            </span>
          </div>
        </div>
      </div>
    );
  }

  // State 3: Mounted & Ready (Selected, waiting for counterpart)
  const folderName = dirPath.split(/[\\/]/).filter(Boolean).pop() || dirPath;

  return (
    <div
      style={{
        flex: 1,
        margin: '16px',
        borderRadius: '12px',
        border: '1px solid var(--border-subtle)',
        background: 'var(--bg-surface)',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        padding: '24px',
        gap: '12px',
      }}
    >
      <div
        style={{
          width: '48px',
          height: '48px',
          borderRadius: '50%',
          background: 'var(--status-success-light)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          color: 'var(--status-success)',
        }}
      >
        <FolderCheck size={24} />
      </div>

      <div style={{ textAlign: 'center', maxWidth: '360px' }}>
        <div style={{ fontSize: '14px', fontWeight: 600, color: 'var(--text-primary)' }}>
          {folderName}
        </div>
        <div
          style={{
            fontSize: '11px',
            color: 'var(--text-tertiary)',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            marginTop: '2px',
          }}
          title={dirPath}
        >
          {dirPath}
        </div>
      </div>

      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '6px',
          fontSize: '11px',
          color: 'var(--status-success)',
          marginTop: '4px',
        }}
      >
        <span style={{ width: '6px', height: '6px', borderRadius: '50%', background: 'var(--status-success)' }} />
        <span>Mounted & Ready</span>
      </div>
    </div>
  );
};

interface SplitPaneStageProps {
  dirA: string | null;
  dirB: string | null;
  isLoading: boolean;
  scanProgress: ScanProgressPayload | null;
  onSelectDirA: () => void;
  onSelectDirB: () => void;
}

export const SplitPaneStage: React.FC<SplitPaneStageProps> = ({
  dirA,
  dirB,
  isLoading,
  scanProgress,
  onSelectDirA,
  onSelectDirB,
}) => {
  return (
    <div
      style={{
        display: 'grid',
        gridTemplateColumns: '1fr 1fr',
        flex: 1,
        overflow: 'hidden',
        background: 'var(--bg-base)',
      }}
    >
      {/* Left Column: Side A Status */}
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          overflow: 'hidden',
          borderRight: '1px solid var(--border-subtle)',
        }}
      >
        <PaneStatusCard
          side="A"
          dirPath={dirA}
          isLoading={isLoading}
          progress={scanProgress?.side_a ?? null}
          onSelect={onSelectDirA}
        />
      </div>

      {/* Right Column: Side B Status */}
      <div style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
        <PaneStatusCard
          side="B"
          dirPath={dirB}
          isLoading={isLoading}
          progress={scanProgress?.side_b ?? null}
          onSelect={onSelectDirB}
        />
      </div>
    </div>
  );
};
