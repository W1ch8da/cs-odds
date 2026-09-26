"use client";

import { MonitorIcon, MoonIcon, SunIcon } from "lucide-react";
import { useTheme } from "next-themes";
import { useSyncExternalStore } from "react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

const ORDER = ["system", "light", "dark"] as const;
const ICONS = { system: MonitorIcon, light: SunIcon, dark: MoonIcon };
const LABELS = { system: "System", light: "Light", dark: "Dark" };

const subscribe = () => () => {};

/** Cycles System → Light → Dark. `withLabel` shows the name next to the icon. */
export function ThemeToggle({ withLabel = false, className }: { withLabel?: boolean; className?: string }) {
  const { theme, setTheme } = useTheme();
  // The theme is only known on the client; render the neutral icon until then.
  const mounted = useSyncExternalStore(subscribe, () => true, () => false);
  const current = (mounted && ORDER.includes(theme as (typeof ORDER)[number]) ? theme : "system") as (typeof ORDER)[number];
  const Icon = ICONS[current];
  const label = `Theme: ${LABELS[current]}`;

  return (
    <Button
      variant="ghost"
      size={withLabel ? "default" : "icon"}
      className={cn(withLabel && "w-full justify-start text-muted-foreground", className)}
      aria-label={label}
      title={label}
      onClick={() => setTheme(ORDER[(ORDER.indexOf(current) + 1) % ORDER.length])}
    >
      <Icon />
      {withLabel && label}
    </Button>
  );
}
