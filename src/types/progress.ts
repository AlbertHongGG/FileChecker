export interface ScanProgressPayload {
  count_a: number;
  count_b: number;
  is_hdd_a: boolean;
  is_hdd_b: boolean;
  phase: string;
}

export interface SyncProgressPayload {
  current: number;
  total: number;
  current_file: string;
}
