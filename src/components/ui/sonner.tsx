import { Toaster as SonnerToaster } from "sonner";

export function Toaster() {
  return (
    <SonnerToaster
      position="top-center"
      richColors
      duration={2000}
      toastOptions={{
        classNames: {
          toast:
            "group rounded-md border bg-background text-foreground shadow-lg",
        },
      }}
    />
  );
}
