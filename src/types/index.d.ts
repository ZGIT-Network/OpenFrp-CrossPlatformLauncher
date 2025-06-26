interface CplUpdate {
  title: string;
  latest: string;
  msg: string;
} 
interface Window {
  __TAURI__?: {
    platform?: string;
    [key: string]: any;
  };
}