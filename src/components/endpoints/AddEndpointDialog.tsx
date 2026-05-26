import { Plus } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { FullScreenPanel } from "@/components/common/FullScreenPanel";
import { api } from "@/lib/api";
import { EndpointForm } from "./forms/EndpointForm";
import type { Endpoint } from "@/types";

interface Props {
  open: boolean;
  onClose: () => void;
  onSaved: () => void;
}

const FORM_ID = "endpoint-add-form";

export function AddEndpointDialog({ open, onClose, onSaved }: Props) {
  return (
    <FullScreenPanel
      open={open}
      onClose={onClose}
      title="添加端点"
      footer={
        <>
          <Button variant="outline" onClick={onClose}>
            取消
          </Button>
          <Button type="submit" form={FORM_ID}>
            <Plus className="h-4 w-4" />
            添加
          </Button>
        </>
      }
    >
      {open && (
        <EndpointForm
          formId={FORM_ID}
          onSubmit={async (values) => {
            try {
              const ep: Endpoint = {
                ...values,
                sortIndex: 0,
                createdAt: 0,
                updatedAt: 0,
              };
              await api.saveEndpoint(ep);
              toast.success("已保存");
              onSaved();
            } catch (e: unknown) {
              toast.error(
                typeof e === "string" ? e : (e as Error)?.message ?? "保存失败",
              );
            }
          }}
        />
      )}
    </FullScreenPanel>
  );
}
