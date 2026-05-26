import { CSSProperties } from "react";
import { DndContext, closestCenter } from "@dnd-kit/core";
import {
  SortableContext,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { EndpointCard } from "./EndpointCard";
import { EndpointEmptyState } from "./EndpointEmptyState";
import { useDragSort } from "@/hooks/useDragSort";
import type { Endpoint, ServerStatus } from "@/types";

interface Props {
  endpoints: Endpoint[];
  status: ServerStatus;
  onEdit: (e: Endpoint) => void;
  onAdd: () => void;
}

export function EndpointList({ endpoints, status, onEdit, onAdd }: Props) {
  const { sortedEndpoints, sensors, handleDragEnd } = useDragSort(endpoints);

  if (endpoints.length === 0) {
    return <EndpointEmptyState onAdd={onAdd} />;
  }

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCenter}
      onDragEnd={handleDragEnd}
    >
      <SortableContext
        items={sortedEndpoints.map((e) => e.id)}
        strategy={verticalListSortingStrategy}
      >
        <div className="mt-4 space-y-4 px-6">
          {sortedEndpoints.map((endpoint) => (
            <SortableEndpointCard
              key={endpoint.id}
              endpoint={endpoint}
              serverRunning={status.running}
              onEdit={() => onEdit(endpoint)}
            />
          ))}
        </div>
      </SortableContext>
    </DndContext>
  );
}

interface SortableProps {
  endpoint: Endpoint;
  serverRunning: boolean;
  onEdit: () => void;
}

function SortableEndpointCard({
  endpoint,
  serverRunning,
  onEdit,
}: SortableProps) {
  const { setNodeRef, attributes, listeners, transform, transition, isDragging } =
    useSortable({ id: endpoint.id });

  const style: CSSProperties = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  return (
    <div ref={setNodeRef} style={style}>
      <EndpointCard
        endpoint={endpoint}
        serverRunning={serverRunning}
        onEdit={onEdit}
        dragHandleProps={{ attributes, listeners, isDragging }}
      />
    </div>
  );
}
