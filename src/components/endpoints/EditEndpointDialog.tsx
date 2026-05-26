import { Save } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { FullScreenPanel } from "@/components/common/FullScreenPanel";
import { api } from "@/lib/api";
import { EndpointForm } from "./forms/EndpointForm";
import type { Endpoint } from "@/types";

interface Props {
  endpoint: Endpoint;
  onClose: () => void;
  onSaved: () => void;
}

const FORM_ID = "endpoint-edit-form";

export function EditEndpointDialog({ endpoint, onClose, onSaved }: Props) {
  return (
    <FullScreenPanel
      open={true}
      onClose={onClose}
      title={`编辑：${endpoint.name}`}
      footer={
        <>
          <Button variant="outline" onClick={onClose}>
            取消
          </Button>
          <Button type="submit" form={FORM_ID}>
            <Save className="h-4 w-4" />
            保存
          </Button>
        </>
      }
    >
      <EndpointForm
        initial={endpoint}
        formId={FORM_ID}
        onSubmit={async (values) => {
          try {
            const ep: Endpoint = {
              ...values,
              sortIndex: endpoint.sortIndex,
              createdAt: endpoint.createdAt,
              updatedAt: endpoint.updatedAt,
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
    </FullScreenPanel>
  );
}
