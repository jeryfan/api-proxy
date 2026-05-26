import { useCallback, useMemo } from "react";
import {
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from "@dnd-kit/core";
import { arrayMove, sortableKeyboardCoordinates } from "@dnd-kit/sortable";
import { toast } from "sonner";
import type { Endpoint } from "@/types";
import { api } from "@/lib/api";

export function useDragSort(endpoints: Endpoint[]) {
  const sortedEndpoints = useMemo(() => {
    return [...endpoints].sort((a, b) => {
      if (a.sortIndex !== b.sortIndex) {
        return a.sortIndex - b.sortIndex;
      }
      const ta = a.createdAt ?? 0;
      const tb = b.createdAt ?? 0;
      if (ta !== tb) return ta - tb;
      return a.name.localeCompare(b.name, "zh-CN");
    });
  }, [endpoints]);

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 8 },
    }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  );

  const handleDragEnd = useCallback(
    async (event: DragEndEvent) => {
      const { active, over } = event;
      if (!over || active.id === over.id) return;

      const oldIndex = sortedEndpoints.findIndex((e) => e.id === active.id);
      const newIndex = sortedEndpoints.findIndex((e) => e.id === over.id);
      if (oldIndex === -1 || newIndex === -1) return;

      const reordered = arrayMove(sortedEndpoints, oldIndex, newIndex);
      const updates = reordered.map((e, idx) => ({ id: e.id, sortIndex: idx }));

      try {
        await api.updateEndpointSort(updates);
      } catch (e: unknown) {
        toast.error(
          typeof e === "string" ? e : (e as Error)?.message ?? "排序更新失败",
        );
      }
    },
    [sortedEndpoints],
  );

  return { sortedEndpoints, sensors, handleDragEnd };
}
