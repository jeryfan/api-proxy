import { invoke } from "@tauri-apps/api/core";
import type { Endpoint, GlobalConfig, InitPayload, ServerStatus } from "@/types";

export const api = {
  initData: () => invoke<InitPayload>("init_data"),
  listEndpoints: () => invoke<Endpoint[]>("list_endpoints"),
  saveEndpoint: (endpoint: Endpoint) =>
    invoke<Endpoint>("save_endpoint", { endpoint }),
  deleteEndpoint: (id: string) => invoke<void>("delete_endpoint", { id }),
  toggleEndpoint: (id: string, enabled: boolean) =>
    invoke<void>("toggle_endpoint", { id, enabled }),
  getGlobalConfig: () => invoke<GlobalConfig>("get_global_config"),
  applyGlobalConfig: (config: GlobalConfig) =>
    invoke<ServerStatus>("apply_global_config", { new: config }),
  serverStatus: () => invoke<ServerStatus>("server_status"),
  startServer: () => invoke<ServerStatus>("start_server"),
  stopServer: () => invoke<ServerStatus>("stop_server"),
  openConfigDir: () => invoke<void>("open_config_dir"),
};

export const EVENTS = {
  STATUS_CHANGED: "proxy://status-changed",
  ENDPOINTS_CHANGED: "proxy://endpoints-changed",
  CONFIG_ERROR: "proxy://config-error",
} as const;
