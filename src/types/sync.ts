export type PlannedAction =
  | 'skip'
  | 'copy_a_to_b'
  | 'copy_b_to_a'
  | 'delete_a'
  | 'delete_b'
  | 'delete_both';

export interface SyncTask {
  relative_path: string;
  action: PlannedAction;
  is_dir?: boolean;
}

export interface SyncResult {
  completed: number;
  total: number;
}
