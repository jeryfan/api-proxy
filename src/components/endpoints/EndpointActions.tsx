import * as React from "react";
import { Copy, Pause, Pencil, Play, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { api } from "@/lib/api";
import type { Endpoint } from "@/types";

interface Props {
  endpoint: Endpoint;
  onEdit: () => void;
}

const iconBtn = "h-8 w-8 p-1";

export function EndpointActions({ endpoint, onEdit }: Props) {
  const [pendingDelete, setPendingDelete] = React.useState(false);
  const [duplicating, setDuplicating] = React.useState(false);

  const toggle = async () => {
    try {
      await api.toggleEndpoint(endpoint.id, !endpoint.enabled);
      toast.success(endpoint.enabled ? "已停用" : "已启用");
    } catch (e: unknown) {
      toast.error(
        typeof e === "string" ? e : (e as Error)?.message ?? "操作失败",
      );
    }
  };

  const duplicate = async () => {
    if (duplicating) return;
    setDuplicating(true);
    try {
      await api.saveEndpoint({
        ...endpoint,
        id: "",
        name: `${endpoint.name} 副本`,
        enabled: false,
        sortIndex: 0,
        createdAt: 0,
        updatedAt: 0,
      });
      toast.success("已复制端点（已停用，请编辑后启用）");
    } catch (e: unknown) {
      toast.error(
        typeof e === "string" ? e : (e as Error)?.message ?? "复制失败",
      );
    } finally {
      setDuplicating(false);
    }
  };

  const remove = async () => {
    setPendingDelete(false);
    try {
      await api.deleteEndpoint(endpoint.id);
      toast.success("已删除");
    } catch (e: unknown) {
      toast.error(
        typeof e === "string" ? e : (e as Error)?.message ?? "删除失败",
      );
    }
  };

  return (
    <>
      <Button
        variant="ghost"
        size="icon"
        className={iconBtn}
        onClick={toggle}
        title={endpoint.enabled ? "停用" : "启用"}
      >
        {endpoint.enabled ? (
          <Pause className="h-4 w-4" />
        ) : (
          <Play className="h-4 w-4" />
        )}
      </Button>
      <Button
        variant="ghost"
        size="icon"
        className={iconBtn}
        onClick={duplicate}
        disabled={duplicating}
        title="复制端点"
      >
        <Copy className="h-4 w-4" />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        className={iconBtn}
        onClick={onEdit}
        title="编辑"
      >
        <Pencil className="h-4 w-4" />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        className={`${iconBtn} hover:text-red-500 dark:hover:text-red-400`}
        onClick={() => setPendingDelete(true)}
        title="删除"
      >
        <Trash2 className="h-4 w-4" />
      </Button>
      <ConfirmDialog
        open={pendingDelete}
        title="删除端点"
        description={`确定要删除 "${endpoint.name}"？此操作不可撤销。`}
        confirmLabel="删除"
        confirmVariant="destructive"
        onCancel={() => setPendingDelete(false)}
        onConfirm={remove}
      />
    </>
  );
}
