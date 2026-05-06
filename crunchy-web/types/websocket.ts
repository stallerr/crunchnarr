export type WsMessageType =
  | 'download_progress'
  | 'download_complete'
  | 'download_failed'
  | 'pong';

export interface WsMessage {
  type: WsMessageType;
  data: unknown;
}

export interface WsDownloadProgress {
  download_id: string;
  phase: string;
  percent: number;
  downloaded_bytes: number;
  total_bytes: number | null;
  speed_bps: number;
  eta_secs: number | null;
  current_step?: number;
  total_steps?: number;
  completed_segments?: number;
  total_segments?: number;
}

export interface WsDownloadComplete {
  download_id: string;
  output_path: string;
}

export interface WsDownloadFailed {
  download_id: string;
  error: string;
}
