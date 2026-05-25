import { GripVertical, Route } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import type { Endpoint } from "@/types";
import { EndpointActions } from "./EndpointActions";

interface Props {
  endpoint: Endpoint;
  serverRunning: boolean;
  listenAddress: string;
  listenPort: number;
  onEdit: () => void;
}

export function EndpointCard({
  endpoint,
  serverRunning,
  listenAddress,
  listenPort,
  onEdit,
}: Props) {
  const clientHost =
    listenAddress === "0.0.0.0" || listenAddress === "::" || !listenAddress
      ? "127.0.0.1"
      : listenAddress;
  const localUrl = `http://${clientHost}:${listenPort}${endpoint.path}`;
  const showInactive = endpoint.enabled && !serverRunning;
  return (
    <div
      className={cn(
        "relative overflow-hidden rounded-xl border p-4 transition-all duration-300",
        "bg-card text-card-foreground group hover:border-border-active hover:shadow-sm",
        endpoint.enabled &&
          "border-emerald-500/60 shadow-sm shadow-emerald-500/10",
      )}
    >
      {endpoint.enabled && (
        <div className="absolute inset-0 bg-gradient-to-r from-emerald-500/10 to-transparent pointer-events-none" />
      )}
      <div className="relative flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex flex-1 items-center gap-2 min-w-0">
          <button
            className="-ml-1.5 flex-shrink-0 cursor-grab p-1.5 text-muted-foreground/50 hover:text-muted-foreground"
            tabIndex={-1}
            aria-hidden
          >
            <GripVertical className="h-4 w-4" />
          </button>
          <div className="h-8 w-8 rounded-lg bg-muted flex items-center justify-center border flex-shrink-0">
            <Route className="h-5 w-5 text-muted-foreground" />
          </div>
          <div className="space-y-1 min-w-0">
            <div className="flex flex-wrap items-center gap-2 min-h-7">
              <h3 className="text-base font-semibold leading-none truncate">
                {endpoint.name}
              </h3>
              <Badge tone={endpoint.enabled ? "violet" : "slate"}>
                {endpoint.enabled ? "已启用" : "已停用"}
              </Badge>
              {showInactive && <Badge tone="amber">服务未运行</Badge>}
            </div>
            <div className="font-mono text-sm truncate max-w-[480px]">
              <span className="text-blue-500 dark:text-blue-400">
                {endpoint.path}
              </span>
              <span className="mx-2 text-muted-foreground">→</span>
              <span className="text-muted-foreground">
                {endpoint.upstreamUrl}
              </span>
            </div>
            {endpoint.description && (
              <div className="text-sm text-muted-foreground truncate max-w-[480px]">
                {endpoint.description}
              </div>
            )}
          </div>
        </div>
        <div
          className={cn(
            "flex items-center gap-1.5 flex-shrink-0",
            "opacity-0 pointer-events-none",
            "group-hover:opacity-100 group-focus-within:opacity-100",
            "group-hover:pointer-events-auto group-focus-within:pointer-events-auto",
            "transition-opacity duration-200",
          )}
        >
          <EndpointActions
            endpoint={endpoint}
            localUrl={localUrl}
            onEdit={onEdit}
          />
        </div>
      </div>
    </div>
  );
}
