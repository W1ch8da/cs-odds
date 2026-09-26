import { Loader2Icon } from "lucide-react";

export function FullPageLoader({ label = "Loading…" }: { label?: string }) {
  return (
    <div className="flex min-h-svh items-center justify-center gap-2 text-sm text-muted-foreground" role="status">
      <Loader2Icon className="size-4 animate-spin" />
      {label}
    </div>
  );
}
