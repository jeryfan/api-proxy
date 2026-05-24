import { Copy } from "lucide-react";
import { toast } from "sonner";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import type { ServerStatus } from "@/types";

export function ServerStatusBadge({ status }: { status: ServerStatus }) {
  const address = status.listenAddress
    ? `${status.listenAddress}:${status.listenPort}`
    : null;
  const copy = async () => {
    if (!address) return;
    await writeText(`http://${address}`);
    toast.success("已复制");
  };
  return (
    <div className="flex items-center gap-2">
      <Badge tone={status.running ? "emerald" : "slate"}>
        {status.running ? "运行中" : "已停止"}
      </Badge>
      {address && (
        <button
          onClick={copy}
          className={cn(
            "inline-flex items-center gap-1 font-mono text-xs",
            "text-muted-foreground hover:text-foreground transition-colors",
          )}
          title="点击复制"
        >
          {address}
          <Copy className="h-3 w-3" />
        </button>
      )}
    </div>
  );
}
