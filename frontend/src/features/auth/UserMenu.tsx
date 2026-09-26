"use client";

import { ChevronsUpDownIcon, LogOutIcon } from "lucide-react";

import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useAppSelector } from "@/lib/store/hooks";
import { cn } from "@/lib/utils";

import { useLogoutMutation } from "./authApi";
import { selectCurrentUser } from "./authSlice";
import { ROLE_LABELS, initials } from "./roles";

/** `compact` shows only the avatar (top bars); otherwise name and role too (sidebars). */
export function UserMenu({ compact = false, side = "bottom" }: { compact?: boolean; side?: "top" | "bottom" }) {
  const user = useAppSelector(selectCurrentUser);
  const [logout, { isLoading }] = useLogoutMutation();
  if (!user) return null;

  const signOut = async () => {
    try {
      await logout().unwrap();
    } finally {
      // A full reload drops every cached query and slice for the previous user,
      // which a client-side navigation would keep in memory.
      // eslint-disable-next-line @next/next/no-location-assign-relative-destination
      window.location.assign("/login");
    }
  };

  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={
          <Button
            variant="ghost"
            className={cn("h-auto gap-2.5 px-2 py-1.5", !compact && "w-full justify-start")}
            aria-label={`Account menu for ${user.name}`}
          />
        }
      >
        <Avatar className="size-7">
          <AvatarFallback className="bg-accent text-xs font-semibold text-accent-foreground">
            {initials(user.name)}
          </AvatarFallback>
        </Avatar>
        {!compact && (
          <>
            <span className="min-w-0 flex-1 text-left leading-tight">
              <span className="block truncate text-sm font-medium">{user.name}</span>
              <span className="block text-xs text-faint">{ROLE_LABELS[user.role]}</span>
            </span>
            <ChevronsUpDownIcon className="text-faint" />
          </>
        )}
        {compact && <span className="hidden text-sm sm:inline">{user.name}</span>}
      </DropdownMenuTrigger>
      <DropdownMenuContent side={side} align={compact ? "end" : "start"} className="w-60">
        <DropdownMenuGroup>
          <DropdownMenuLabel className="flex flex-col gap-0.5 py-1.5">
            <span className="text-sm font-medium text-foreground">{user.name}</span>
            <span className="truncate font-normal">{user.email}</span>
          </DropdownMenuLabel>
        </DropdownMenuGroup>
        <DropdownMenuSeparator />
        <DropdownMenuItem onClick={signOut} disabled={isLoading}>
          <LogOutIcon />
          Sign out
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
