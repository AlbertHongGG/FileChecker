export interface ScanProgressPayload {
  completed_a: number;
  total_a: number;
  current_name_a: string;
  count_a: number;
  is_hdd_a: boolean;
  engine_a?: string;

  completed_b: number;
  total_b: number;
  current_name_b: string;
  count_b: number;
  is_hdd_b: boolean;
  engine_b?: string;

  phase: string;
}

export interface SyncProgressPayload {
  current: number;
  total: number;
  current_file: string;
}
