import { DiffKind, FileMetadata } from './entry';

export interface DescendantEntry {
  path: string;
  diffType: DiffKind['type'];
  isDir: boolean;
}

export type DirectoryActionType = 'mirror_a_to_b' | 'mirror_b_to_a' | 'delete' | 'skip';

export interface TreeNode {
  relative_path: string;
  name: string;
  is_dir: boolean;
  diff_kind: DiffKind;
  meta_a?: FileMetadata | null;
  meta_b?: FileMetadata | null;
  children?: TreeNode[];
  modifiableFiles?: string[];
  descendantEntries?: DescendantEntry[];
}

export interface FlattenedRow {
  node: TreeNode;
  depth: number;
  hasChildren: boolean;
  isExpanded: boolean;
}
