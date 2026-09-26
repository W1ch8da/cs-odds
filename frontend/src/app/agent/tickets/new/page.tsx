import type { Metadata } from "next";

import { NewTicketForm } from "@/features/tickets/components/NewTicketForm";

export const metadata: Metadata = { title: "New ticket" };

export default function NewTicketPage() {
  return <NewTicketForm />;
}
