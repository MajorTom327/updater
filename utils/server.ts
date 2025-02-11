#!/usr/bin/env deno run --allow-read --allow-write --allow-env --allow-net --allow-run

import { serve } from "https://deno.land/std/http/server.ts";

const handler = (req: Request): Response => {
  const now = new Date();
  const minutes = now.getMinutes();
  const roundedMinutes = Math.floor(minutes / 5) * 5;
  now.setMinutes(roundedMinutes);
  now.setSeconds(0);
  now.setMilliseconds(0);
  const buildAt = now.toISOString();
  return new Response(JSON.stringify({ buildAt }), {
    headers: { "Content-Type": "application/json" },
  });
};

const port = Number(Deno.args[0]) || 8000;
console.log(`Starting server on port ${port}`);
serve(handler, { port });
