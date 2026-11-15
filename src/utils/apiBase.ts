/**
 * 当前可用的 OpenFrp API 地址（主地址 + 备用地址）。
 * 与 Rust 端 src-tauri/src/api_proxy.rs 中的 OPENFRP_API_BASES 保持一致。
 */
export const OPENFRP_API_BASES = [
  'https://api.openfrp.net',
  'https://ofapi.naids.com',
] as const;
