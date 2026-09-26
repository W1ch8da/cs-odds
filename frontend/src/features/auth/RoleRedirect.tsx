"use client";

import { useRouter } from "next/navigation";
import { useEffect } from "react";

import { FullPageLoader } from "@/components/layout/FullPageLoader";
import { errorStatus } from "@/lib/api-error";

import { useMeQuery } from "./authApi";
import { homeFor } from "./roles";

/** Sends the visitor to the right home for their role, or to sign in. */
export function RoleRedirect() {
  const { data: user, error } = useMeQuery();
  const router = useRouter();

  useEffect(() => {
    if (user) router.replace(homeFor(user.role));
    else if (errorStatus(error) === 401) router.replace("/login");
  }, [user, error, router]);

  return <FullPageLoader />;
}
