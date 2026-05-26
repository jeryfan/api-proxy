import { invoke } from "@tauri-apps/api/core";
import type {
  DetectedProxy,
  Endpoint,
  GlobalConfig,
  InitPayload,
  ProxyTestResult,
  RequestLog,
  ServerStatus,
} from "@/types";

export const api = {
  initData: () => invoke<InitPayload>("init_data"),
  listEndpoints: () => invoke<Endpoint[]>("list_endpoints"),
  saveEndpoint: (endpoint: Endpoint) =>
    invoke<Endpoint>("save_endpoint", { endpoint }),
  deleteEndpoint: (id: string) => invoke<void>("delete_endpoint", { id }),
  toggleEndpoint: (id: string, enabled: boolean) =>
    invoke<void>("toggle_endpoint", { id, enabled }),
  updateEndpointSort: (updates: { id: string; sortIndex: number }[]) =>
    invoke<void>("update_endpoint_sort", { updates }),
  getGlobalConfig: () => invoke<GlobalConfig>("get_global_config"),
  applyGlobalConfig: (config: GlobalConfig) =>
    invoke<ServerStatus>("apply_global_config", { new: config }),
  serverStatus: () => invoke<ServerStatus>("server_status"),
  startServer: () => invoke<ServerStatus>("start_server"),
  stopServer: () => invoke<ServerStatus>("stop_server"),
  openConfigDir: () => invoke<void>("open_config_dir"),
  getGlobalProxyUrl: () => invoke<string>("get_global_proxy_url"),
  setGlobalProxyUrl: (url: string) =>
    invoke<void>("set_global_proxy_url", { url }),
  testProxyUrl: (url: string) =>
    invoke<ProxyTestResult>("test_proxy_url", { url }),
  scanLocalProxies: () => invoke<DetectedProxy[]>("scan_local_proxies"),
  listRequestLogs: (endpointId?: string, limit?: number) =>
    invoke<RequestLog[]>("list_request_logs", { endpointId, limit }),
  getRequestLog: (id: string) =>
    invoke<RequestLog | null>("get_request_log", { id }),
  clearRequestLogs: (endpointId?: string) =>
    invoke<void>("clear_request_logs", { endpointId }),
};

export const EVENTS = {
  STATUS_CHANGED: "proxy://status-changed",
  ENDPOINTS_CHANGED: "proxy://endpoints-changed",
  CONFIG_ERROR: "proxy://config-error",
  LOG_RECORDED: "proxy://log-recorded",
} as const;
