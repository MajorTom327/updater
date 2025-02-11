#!/usr/bin/env deno run --allow-read --allow-write --allow-env --allow-net --allow-run

import { serve } from "https://deno.land/std/http/server.ts";

const handler = (req: Request): Response => {
  const buildAt = new Date().toISOString();
  return new Response(JSON.stringify({ buildAt }), {
    headers: { "Content-Type": "application/json" },
  });
};

serve(handler, { port: 8000 });
