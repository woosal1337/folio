/**
 * Bearer-token auth middleware. Mounts before every protected route.
 * Sets c.var.user_id from the JWT sub claim. Throws 401 on failure.
 */

import type { MiddlewareHandler } from "hono";
import { HTTPException } from "hono/http-exception";
import type { Env } from "./env";
import { verifySession } from "./jwt";

export const requireAuth: MiddlewareHandler<Env> = async (c, next) => {
  const header = c.req.header("Authorization");
  if (!header || !header.startsWith("Bearer ")) {
    throw new HTTPException(401, { message: "missing_bearer_token" });
  }
  const token = header.slice("Bearer ".length).trim();
  if (!token) {
    throw new HTTPException(401, { message: "empty_bearer_token" });
  }
  try {
    const claims = await verifySession(c.env, token);
    c.set("user_id", claims.sub);
    await next();
  } catch (e) {
    const msg = e instanceof Error ? e.message : "invalid_token";
    throw new HTTPException(401, { message: `auth_failed:${msg}` });
  }
};
