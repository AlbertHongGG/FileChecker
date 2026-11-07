import { DiffKind, FileMetadata } from './entry';

export interface TreeNode {
  relative_path: string;
  name: string;
  is_dir: boolean;
  diff_kind: DiffKind;
  meta_a?: FileMetadata | null;
  meta_b?: FileMetadata | null;
  children?: TreeNode[];
  // Directory capabilities computed bottom-up
  canCopyAtoB?: boolean;
  canCopyBtoA?: boolean;
  canDelete?: boolean;
  modifiableFiles?: string[];
  descendantFiles?: Array<{ path: string; diffType: DiffKind['type'] }>;
}

export interface FlattenedRow {
  node: TreeNode;
  depth: number;
  hasChildren: boolean;
  isExpanded: boolean;
}
