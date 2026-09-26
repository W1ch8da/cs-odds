import { NextResponse, type NextRequest } from "next/server";

// Set by the Rust API as httpOnly cookies. Only their presence is checked here;
// the API verifies them on every request.
const REFRESH_COOKIE = "cs_refresh";

const AUTH_PAGES = ["/login", "/register"];

/**
 * Optimistic routing before render: signed-out visitors go straight to the
 * login page, and signed-in visitors skip it. Role checks happen in
 * RequireRole, and real authorization in the API.
 */
export function proxy(request: NextRequest) {
  const { pathname, search } = request.nextUrl;
  const hasSession = request.cookies.has(REFRESH_COOKIE);

  if (AUTH_PAGES.includes(pathname)) {
    return hasSession ? NextResponse.redirect(new URL("/", request.url)) : NextResponse.next();
  }

  if (!hasSession) {
    const login = new URL("/login", request.url);
    if (pathname !== "/") login.searchParams.set("next", pathname + search);
    return NextResponse.redirect(login);
  }
  return NextResponse.next();
}

export const config = {
  matcher: ["/", "/login", "/register", "/portal/:path*", "/agent/:path*", "/admin/:path*"],
};
