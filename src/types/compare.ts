export type DiffType = 'Identical' | 'OnlyInA' | 'OnlyInB' | 'Modified';

export interface DiffKind {
  type: DiffType;
  a_newer?: boolean;
  b_newer?: boolean;
  same_time?: boolean;
}

export interface FileMetadata {
  size: number;
  modified_timestamp: number;
  is_dir: boolean;
}

export interface UnifiedEntry {
  relative_path: string;
  name: string;
  is_dir: boolean;
  diff_kind: DiffKind;
  meta_a?: FileMetadata | null;
  meta_b?: FileMetadata | null;
}

export type PlannedAction = 'skip' | 'copy_a_to_b' | 'copy_b_to_a' | 'delete_a' | 'delete_b';

export interface SyncTask {
  relative_path: string;
  action: PlannedAction;
}

export interface SyncProgressPayload {
  current: number;
  total: number;
  current_file: string;
}

export interface TreeNode {
  relative_path: string;
  name: string;
  is_dir: boolean;
  diff_kind: DiffKind;
  meta_a?: FileMetadata | null;
  meta_b?: FileMetadata | null;
  children?: TreeNode[];
}
