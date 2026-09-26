import type { Metadata } from "next";

import { NewRequestForm } from "@/features/portal/NewRequestForm";

export const metadata: Metadata = { title: "New request" };

export default function NewRequestPage() {
  return <NewRequestForm />;
}
