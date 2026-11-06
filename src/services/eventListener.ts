import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { ScanProgressPayload, SyncProgressPayload } from '../types';

export class EventListenerService {
  static async onScanProgress(
    callback: (payload: ScanProgressPayload) => void
  ): Promise<UnlistenFn> {
    return listen<ScanProgressPayload>('scan-progress', (event) => {
      callback(event.payload);
    });
  }

  static async onSyncProgress(
    callback: (payload: SyncProgressPayload) => void
  ): Promise<UnlistenFn> {
    return listen<SyncProgressPayload>('sync-progress', (event) => {
      callback(event.payload);
    });
  }
}
