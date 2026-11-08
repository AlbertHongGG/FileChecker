export type ScanStage =
  | { kind: 'Initializing' }
  | { kind: 'ReadingMftBlocks'; data: { completed_blocks: number; total_blocks: number } }
  | { kind: 'BuildingIndex'; data: { records_parsed: number } }
  | { kind: 'ExtractingSubtree'; data: { files_indexed: number } }
  | { kind: 'WalkingDirectories'; data: { completed_dirs: number; total_dirs: number; files_scanned: number; current_dir: string } }
  | { kind: 'Completed'; data: { total_files: number } }
  | { kind: 'Cancelled' }
  | { kind: 'Failed'; data: { error: string } };

export interface SideScanProgress {
  engine: string;
  is_hdd: boolean;
  percent: number;
  stage: ScanStage;
}

export interface ScanProgressPayload {
  side_a: SideScanProgress;
  side_b: SideScanProgress;
}

export interface SyncProgressPayload {
  current: number;
  total: number;
  current_file: string;
}
