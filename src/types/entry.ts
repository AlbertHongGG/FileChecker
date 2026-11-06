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
