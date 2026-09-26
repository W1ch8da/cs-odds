import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { initials } from "@/features/auth/roles";
import { cn } from "@/lib/utils";

/** Initials avatar. `highlight` marks the signed-in user or the support team. */
export function PersonAvatar({ name, highlight = false, className }: { name: string; highlight?: boolean; className?: string }) {
  return (
    <Avatar className={cn("size-7", className)}>
      <AvatarFallback
        className={cn("text-[11px] font-semibold", highlight ? "bg-accent text-accent-foreground" : "bg-muted text-muted-foreground")}
      >
        {initials(name)}
      </AvatarFallback>
    </Avatar>
  );
}
