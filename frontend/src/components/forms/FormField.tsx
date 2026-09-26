import { CircleAlertIcon } from "lucide-react";

import { Label } from "@/components/ui/label";

type Props = {
  id: string;
  label: string;
  hint?: string;
  error?: string;
  children: React.ReactNode;
};

/** Label, control, optional hint and inline error, wired up for screen readers. */
export function FormField({ id, label, hint, error, children }: Props) {
  return (
    <div className="flex flex-col gap-1.5">
      <Label htmlFor={id}>{label}</Label>
      {children}
      {hint && !error && (
        <p id={`${id}-hint`} className="text-xs text-muted-foreground">
          {hint}
        </p>
      )}
      {error && (
        <p id={`${id}-error`} role="alert" className="flex items-center gap-1.5 text-xs text-destructive">
          <CircleAlertIcon className="size-3.5 shrink-0" />
          {error}
        </p>
      )}
    </div>
  );
}

/** Props to spread on the control inside a FormField. */
export function fieldA11y(id: string, error?: string, hint?: string) {
  return {
    id,
    "aria-invalid": error ? true : undefined,
    "aria-describedby": error ? `${id}-error` : hint ? `${id}-hint` : undefined,
  };
}
