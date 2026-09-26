import { PortalShell } from "@/components/layout/PortalShell";
import { RequireRole } from "@/features/auth/RequireRole";

export default function PortalLayout({ children }: LayoutProps<"/portal">) {
  return (
    <RequireRole roles={["customer"]}>
      <PortalShell>{children}</PortalShell>
    </RequireRole>
  );
}
