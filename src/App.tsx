import * as React from "react";
import { Plus, Settings as SettingsIcon } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { ModeToggle } from "@/components/mode-toggle";
import { useTheme } from "@/components/theme-provider";
import { api, EVENTS } from "@/lib/api";
import { isMac } from "@/lib/platform";
import type { Endpoint, GlobalConfig, ServerStatus } from "@/types";

import { ServerToggle } from "@/components/ServerToggle";
import { ServerStatusBadge } from "@/components/ServerStatusBadge";
import { EndpointList } from "@/components/endpoints/EndpointList";
import { AddEndpointDialog } from "@/components/endpoints/AddEndpointDialog";
import { EditEndpointDialog } from "@/components/endpoints/EditEndpointDialog";
import { SettingsPanel } from "@/components/settings/SettingsPanel";

const DRAG_BAR = isMac() ? 28 : 0;
const HEADER_H = 64;

export default function App() {
  const { setTheme } = useTheme();
  const [endpoints, setEndpoints] = React.useState<Endpoint[]>([]);
  const [global, setGlobal] = React.useState<GlobalConfig | null>(null);
  const [status, setStatus] = React.useState<ServerStatus>({ running: false });
  const [showAdd, setShowAdd] = React.useState(false);
  const [editing, setEditing] = React.useState<Endpoint | null>(null);
  const [showSettings, setShowSettings] = React.useState(false);

  const refresh = React.useCallback(async () => {
    const data = await api.initData();
    setEndpoints(data.endpoints);
    setGlobal(data.global);
    setStatus(data.status);
    setTheme(data.global.theme);
  }, [setTheme]);

  React.useEffect(() => {
    refresh().catch((e) => toast.error(`初始化失败：${e}`));
    getCurrentWindow().show().catch(() => undefined);
  }, [refresh]);

  React.useEffect(() => {
    const unlisten1 = listen<ServerStatus>(EVENTS.STATUS_CHANGED, (e) =>
      setStatus(e.payload),
    );
    const unlisten2 = listen<Endpoint[]>(EVENTS.ENDPOINTS_CHANGED, (e) =>
      setEndpoints(e.payload),
    );
    const unlisten3 = listen<{ message: string }>(EVENTS.CONFIG_ERROR, (e) =>
      toast.error(e.payload.message),
    );
    return () => {
      unlisten1.then((f) => f());
      unlisten2.then((f) => f());
      unlisten3.then((f) => f());
    };
  }, []);

  if (!global) {
    return (
      <div className="flex h-screen items-center justify-center text-muted-foreground">
        加载中…
      </div>
    );
  }

  return (
    <div
      className="flex flex-col h-screen overflow-hidden bg-background text-foreground selection:bg-primary/30 pb-4"
      style={{ paddingTop: DRAG_BAR + HEADER_H }}
    >
      {DRAG_BAR > 0 && (
        <div
          data-tauri-drag-region
          style={{ height: DRAG_BAR }}
          className="fixed left-0 right-0 top-0 z-[70]"
        />
      )}

      <header
        data-tauri-drag-region
        className="fixed left-0 right-0 z-50 bg-background/80 backdrop-blur-md"
        style={{ top: DRAG_BAR, height: HEADER_H }}
      >
        <div className="flex h-full items-center justify-between gap-2 px-6">
          <div className="flex items-center gap-2">
            <span className="text-xl font-semibold text-blue-500 dark:text-blue-400">
              API 代理
            </span>
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={() => setShowSettings(true)}
              data-tauri-no-drag
              title="设置"
            >
              <SettingsIcon className="h-4 w-4" />
            </Button>
          </div>
          <div className="flex items-center gap-3" data-tauri-no-drag>
            <ServerStatusBadge status={status} />
            <ServerToggle status={status} />
            <ModeToggle />
            <Button
              onClick={() => setShowAdd(true)}
              size="icon"
              className="ml-2"
            >
              <Plus className="h-5 w-5" />
            </Button>
          </div>
        </div>
      </header>

      <main className="flex-1 min-h-0 flex flex-col overflow-y-auto animate-fade-in">
        <EndpointList
          endpoints={endpoints}
          status={status}
          listenAddress={status.listenAddress ?? global.listenAddress}
          listenPort={status.listenPort ?? global.listenPort}
          onEdit={setEditing}
          onAdd={() => setShowAdd(true)}
        />
      </main>

      <AddEndpointDialog
        open={showAdd}
        onClose={() => setShowAdd(false)}
        onSaved={() => setShowAdd(false)}
      />
      {editing && (
        <EditEndpointDialog
          endpoint={editing}
          onClose={() => setEditing(null)}
          onSaved={() => setEditing(null)}
        />
      )}
      <SettingsPanel
        open={showSettings}
        global={global}
        onClose={() => setShowSettings(false)}
        onSaved={(g) => {
          setGlobal(g);
          setTheme(g.theme);
        }}
      />
    </div>
  );
}
