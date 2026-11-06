import { invoke } from '@tauri-apps/api/core';
import { UnifiedEntry, SyncTask } from '../types';

export class TauriClient {
  static async compareDirectories(
    dirA: string,
    dirB: string,
    pruneIgnored: boolean
  ): Promise<UnifiedEntry[]> {
    return invoke<UnifiedEntry[]>('compare_directories', {
      dirA,
      dirB,
      pruneIgnored,
    });
  }

  static async cancelComparison(): Promise<void> {
    return invoke<void>('cancel_comparison');
  }

  static async executeSync(
    dirA: string,
    dirB: string,
    tasks: SyncTask[]
  ): Promise<number> {
    return invoke<number>('execute_sync', {
      dirA,
      dirB,
      tasks,
    });
  }

  static async getFileContentPair(
    dirA: string,
    dirB: string,
    relativePath: string
  ): Promise<[string, string]> {
    return invoke<[string, string]>('get_file_content_pair', {
      dirA,
      dirB,
      relativePath,
    });
  }

  static async revealInExplorer(fullPath: string): Promise<void> {
    return invoke<void>('reveal_in_explorer', { fullPath });
  }
}
