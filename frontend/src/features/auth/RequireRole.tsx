"use client";

import { RefreshCwIcon, WifiOffIcon } from "lucide-react";
import { usePathname, useRouter } from "next/navigation";
import { useEffect } from "react";

import { EmptyState } from "@/components/layout/EmptyState";
import { FullPageLoader } from "@/components/layout/FullPageLoader";
import { Button } from "@/components/ui/button";
import { errorMessage, errorStatus } from "@/lib/api-error";

import { useMeQuery } from "./authApi";
import { homeFor } from "./roles";
import type { Role } from "./types";

/**
 * Renders children only for signed-in users with one of `roles`. Signed-out
 * users go to the login page; users with another role go to their own home.
 * This is for navigation only: the API enforces access on every request.
 */
export function RequireRole({ roles, children }: { roles: Role[]; children: React.ReactNode }) {
  const { data: user, error, isError, refetch, isFetching } = useMeQuery();
  const router = useRouter();
  const pathname = usePathname();
  const signedOut = isError && errorStatus(error) === 401;
  const allowed = user && roles.includes(user.role);

  useEffect(() => {
    if (signedOut) router.replace(`/login?next=${encodeURIComponent(pathname)}`);
    else if (user && !roles.includes(user.role)) router.replace(homeFor(user.role));
  }, [signedOut, user, roles, router, pathname]);

  if (allowed) return children;

  if (isError && !signedOut) {
    return (
      <div className="mx-auto flex min-h-svh max-w-md items-center px-4">
        <EmptyState
          icon={WifiOffIcon}
          title="We couldn't load your account"
          action={
            <Button variant="outline" onClick={() => refetch()} disabled={isFetching}>
              <RefreshCwIcon />
              Try again
            </Button>
          }
        >
          {errorMessage(error)}
        </EmptyState>
      </div>
    );
  }

  return <FullPageLoader />;
}
