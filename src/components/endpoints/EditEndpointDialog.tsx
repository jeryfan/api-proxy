import { Save } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { FullScreenPanel } from "@/components/common/FullScreenPanel";
import { api } from "@/lib/api";
import { EndpointForm } from "./forms/EndpointForm";
import type { Endpoint, UpstreamHealthState } from "@/types";

interface Props {
  endpoint: Endpoint;
  healthStates?: Record<string, UpstreamHealthState>;
  onClose: () => void;
  onSaved: () => void;
}

const FORM_ID = "endpoint-edit-form";

export function EditEndpointDialog({ endpoint, healthStates, onClose, onSaved }: Props) {
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
        healthStates={healthStates}
        onResetHealth={async (upstreamId) => {
          try {
            await api.resetUpstreamHealth(endpoint.id, upstreamId);
            toast.success("已重置并恢复该上游");
          } catch (e: unknown) {
            toast.error(typeof e === "string" ? e : (e as Error)?.message ?? "重置失败");
          }
        }}
        onSubmit={async (values) => {
          try {
            const ep: Endpoint = {
              ...values,
              sortIndex: endpoint.sortIndex,
              createdAt: endpoint.createdAt,
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
