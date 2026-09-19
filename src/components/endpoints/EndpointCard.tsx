import type {
  DraggableAttributes,
  DraggableSyntheticListeners,
} from "@dnd-kit/core";
import { AlertTriangle, GripVertical, RotateCcw, Route } from "lucide-react";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { api } from "@/lib/api";
import { cn } from "@/lib/utils";
import type { Endpoint, UpstreamHealthState } from "@/types";
import { EndpointActions } from "./EndpointActions";

interface DragHandleProps {
  attributes: DraggableAttributes;
  listeners: DraggableSyntheticListeners;
  isDragging: boolean;
}

interface Props {
  endpoint: Endpoint;
  serverRunning: boolean;
  healthStates?: Record<string, UpstreamHealthState>;
  onEdit: () => void;
  onViewLogs: () => void;
  dragHandleProps?: DragHandleProps;
}

export function EndpointCard({
  endpoint,
  serverRunning,
  healthStates,
  onEdit,
  onViewLogs,
  dragHandleProps,
}: Props) {
  const showInactive = endpoint.enabled && !serverRunning;
  const isDragging = dragHandleProps?.isDragging ?? false;

  const trippedUpstreams = endpoint.upstreams.filter((u) => {
    const hs = healthStates?.[`${endpoint.id}:${u.id}`];
    return hs?.isTripped;
  });

  return (
    <div
      className={cn(
        "relative overflow-hidden rounded-xl border p-4 transition-all duration-300",
        "bg-card text-card-foreground group hover:border-primary hover:shadow-sm",
        endpoint.enabled &&
          "border-emerald-500/60 shadow-sm shadow-emerald-500/10",
        trippedUpstreams.length > 0 &&
          "border-amber-500/60 shadow-sm shadow-amber-500/10",
        isDragging && "cursor-grabbing border-primary shadow-lg scale-105 z-10",
      )}
    >
      {endpoint.enabled && (
        <div className="absolute inset-0 bg-gradient-to-r from-emerald-500/10 to-transparent pointer-events-none" />
      )}
      <div className="relative flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex flex-1 items-center gap-2 min-w-0">
          <button
            type="button"
            className="-ml-1.5 flex-shrink-0 cursor-grab active:cursor-grabbing p-1.5 text-muted-foreground/50 hover:text-muted-foreground transition-colors"
            {...(dragHandleProps?.attributes ?? {})}
            {...(dragHandleProps?.listeners ?? {})}
          >
            <GripVertical className="h-4 w-4" />
          </button>
          <div className="h-8 w-8 rounded-lg bg-muted flex items-center justify-center border flex-shrink-0 group-hover:scale-105 transition-transform duration-300">
            <Route className="h-5 w-5 text-muted-foreground" />
          </div>
          <div className="space-y-1 min-w-0 flex-1">
            <div className="flex flex-wrap items-center gap-2 min-h-7">
              <h3 className="text-base font-semibold leading-none truncate">
                {endpoint.name}
              </h3>
              <Badge tone={endpoint.enabled ? "violet" : "slate"}>
                {endpoint.enabled ? "已启用" : "已停用"}
              </Badge>
              {showInactive && <Badge tone="amber">服务未运行</Badge>}
              {trippedUpstreams.length > 0 && (
                <Badge tone="red" className="gap-1 font-medium">
                  <AlertTriangle className="h-3 w-3" />
                  {trippedUpstreams.length} 个上游已熔断
                </Badge>
              )}
            </div>
            <div className="font-mono text-sm truncate max-w-[480px]">
              <span className="text-blue-500 dark:text-blue-400">
                {endpoint.path}
              </span>
              <span className="mx-2 text-muted-foreground">→</span>
              <span className="text-muted-foreground">
                {endpoint.upstreams[0]?.url ?? "（无上游）"}
              </span>
              {endpoint.upstreams.length > 1 && (
                <Badge tone="slate" className="ml-2">
                  共 {endpoint.upstreams.length} 个
                </Badge>
              )}
            </div>
            {endpoint.description && (
              <div className="text-sm text-muted-foreground truncate max-w-[480px]">
                {endpoint.description}
              </div>
            )}

            {/* 熔断告警与一键恢复栏 */}
            {trippedUpstreams.length > 0 && (
              <div className="mt-2 space-y-1.5 pt-1">
                {trippedUpstreams.map((u) => {
                  const hs = healthStates?.[`${endpoint.id}:${u.id}`];
                  return (
                    <div
                      key={u.id}
                      className="flex items-center justify-between rounded-md border border-rose-500/30 bg-rose-500/10 px-2.5 py-1 text-xs text-rose-600 dark:text-rose-400"
                    >
                      <div className="flex items-center gap-1.5 min-w-0 pr-2">
                        <AlertTriangle className="h-3.5 w-3.5 flex-shrink-0" />
                        <span className="truncate">
                          <strong>{u.name || u.url}</strong>: 熔断失效
                          {hs?.lastFailureReason
                            ? ` (${hs.lastFailureReason})`
                            : `（连续 ${hs?.consecutiveFailures} 次失败）`}
                        </span>
                      </div>
                      <Button
                        size="sm"
                        variant="outline"
                        className="h-6 px-2 text-[11px] flex-shrink-0 border-rose-500/30 hover:bg-rose-500/20"
                        onClick={async (e) => {
                          e.stopPropagation();
                          try {
                            await api.resetUpstreamHealth(endpoint.id, u.id);
                            toast.success(`已恢复上游 ${u.name || u.url}`);
                          } catch (err) {
                            toast.error(`恢复失败: ${err}`);
                          }
                        }}
                      >
                        <RotateCcw className="h-3 w-3 mr-1" />
                        手动开启
                      </Button>
                    </div>
                  );
                })}
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
            onEdit={onEdit}
            onViewLogs={onViewLogs}
          />
        </div>
      </div>
    </div>
  );
}
