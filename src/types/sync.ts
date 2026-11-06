export type PlannedAction =
  | 'skip'
  | 'copy_a_to_b'
  | 'copy_b_to_a'
  | 'delete_a'
  | 'delete_b';

export interface SyncTask {
  relative_path: string;
  action: PlannedAction;
}

export interface SyncResult {
  completed: number;
  total: number;
}
