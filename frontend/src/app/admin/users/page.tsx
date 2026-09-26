import type { Metadata } from "next";

import { TeamAndCustomers } from "@/features/users/TeamAndCustomers";

export const metadata: Metadata = { title: "Team & customers" };

export default function UsersPage() {
  return <TeamAndCustomers />;
}
