import { toast } from "sonner";
import { Switch } from "@/components/ui/switch";
import { api } from "@/lib/api";
import type { ServerStatus } from "@/types";

export function ServerToggle({ status }: { status: ServerStatus }) {
  const handleChange = async (checked: boolean) => {
    try {
      if (checked) {
        await api.startServer();
        toast.success("服务已启动");
      } else {
        await api.stopServer();
        toast.success("服务已停止");
      }
    } catch (e: unknown) {
      toast.error(
        typeof e === "string" ? e : (e as Error)?.message ?? "操作失败",
      );
    }
  };

  const tooltip = status.running
    ? `运行中 ${status.listenAddress}:${status.listenPort}`
    : "已停止";

  return (
    <Switch
      checked={status.running}
      onCheckedChange={handleChange}
      title={tooltip}
    />
  );
}
