import { Plus, Route } from "lucide-react";
import { Button } from "@/components/ui/button";

export function EndpointEmptyState({ onAdd }: { onAdd: () => void }) {
  return (
    <div className="m-6 flex flex-col items-center justify-center rounded-lg border border-dashed p-10 text-center">
      <div className="mb-4 flex h-16 w-16 items-center justify-center rounded-full bg-muted">
        <Route className="h-7 w-7 text-muted-foreground" />
      </div>
      <h3 className="text-lg font-semibold">还没有任何端点</h3>
      <p className="mt-2 max-w-lg text-sm text-muted-foreground">
        点击下面的按钮创建第一个端点，把请求转发到上游 API。
      </p>
      <div className="mt-6">
        <Button onClick={onAdd}>
          <Plus className="h-4 w-4" />
          添加端点
        </Button>
      </div>
    </div>
  );
}
