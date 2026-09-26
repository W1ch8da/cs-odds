import type { Metadata } from "next";

import { PortalHome } from "@/features/portal/PortalHome";

export const metadata: Metadata = { title: "My requests" };

export default function PortalPage() {
  return <PortalHome />;
}
