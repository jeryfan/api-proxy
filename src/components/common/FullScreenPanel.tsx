import { AnimatePresence, motion } from "framer-motion";
import { ArrowLeft } from "lucide-react";
import * as React from "react";
import { createPortal } from "react-dom";
import { Button } from "@/components/ui/button";
import {
  DRAG_REGION_ATTR,
  DRAG_REGION_ENABLED,
  DRAG_REGION_STYLE,
  isMac,
} from "@/lib/platform";
import { cn } from "@/lib/utils";

interface FullScreenPanelProps {
  open: boolean;
  onClose: () => void;
  title: string;
  footer?: React.ReactNode;
  children: React.ReactNode;
}

export function FullScreenPanel({
  open,
  onClose,
  title,
  footer,
  children,
}: FullScreenPanelProps) {
  React.useEffect(() => {
    if (!open) return;
    const prev = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    const handler = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      const active = document.activeElement as HTMLElement | null;
      if (
        active &&
        (active.tagName === "INPUT" ||
          active.tagName === "TEXTAREA" ||
          active.isContentEditable)
      ) {
        return;
      }
      e.preventDefault();
      onClose();
    };
    window.addEventListener("keydown", handler);
    return () => {
      document.body.style.overflow = prev;
      window.removeEventListener("keydown", handler);
    };
  }, [open, onClose]);

  if (typeof document === "undefined") return null;

  const dragBarHeight = isMac() && DRAG_REGION_ENABLED ? 28 : 0;

  return createPortal(
    <AnimatePresence>
      {open && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.2 }}
          className="fixed inset-0 z-[60] flex flex-col bg-background"
        >
          {dragBarHeight > 0 && (
            <div
              data-tauri-drag-region
              className="w-full shrink-0"
              style={
                {
                  WebkitAppRegion: "drag",
                  height: dragBarHeight,
                } as React.CSSProperties
              }
            />
          )}
          <header
            {...DRAG_REGION_ATTR}
            className={cn(
              "flex h-16 shrink-0 items-center gap-3 px-6 border-b",
              "bg-background/95 backdrop-blur",
            )}
            style={DRAG_REGION_STYLE as React.CSSProperties}
          >
            <div
              className="flex items-center gap-3"
              style={{ WebkitAppRegion: "no-drag" } as React.CSSProperties}
            >
              <Button
                variant="outline"
                size="icon"
                className="rounded-lg"
                onClick={onClose}
              >
                <ArrowLeft className="h-4 w-4" />
              </Button>
              <h1 className="text-lg font-semibold">{title}</h1>
            </div>
          </header>
          <main className="flex-1 overflow-y-auto px-6 py-6 space-y-6">
            {children}
          </main>
          {footer && (
            <footer className="flex shrink-0 items-center justify-end gap-2 px-6 py-4 border-t bg-background/95 backdrop-blur">
              {footer}
            </footer>
          )}
        </motion.div>
      )}
    </AnimatePresence>,
    document.body,
  );
}
