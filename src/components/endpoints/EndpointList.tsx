import { EndpointCard } from "./EndpointCard";
import { EndpointEmptyState } from "./EndpointEmptyState";
import type { Endpoint, ServerStatus } from "@/types";

interface Props {
  endpoints: Endpoint[];
  status: ServerStatus;
  listenAddress: string;
  listenPort: number;
  onEdit: (e: Endpoint) => void;
  onAdd: () => void;
}

export function EndpointList({
  endpoints,
  status,
  listenAddress,
  listenPort,
  onEdit,
  onAdd,
}: Props) {
  if (endpoints.length === 0) {
    return <EndpointEmptyState onAdd={onAdd} />;
  }
  return (
    <div className="mt-4 space-y-4 px-6">
      {endpoints.map((e) => (
        <EndpointCard
          key={e.id}
          endpoint={e}
          serverRunning={status.running}
          listenAddress={listenAddress}
          listenPort={listenPort}
          onEdit={() => onEdit(e)}
        />
      ))}
    </div>
  );
}
