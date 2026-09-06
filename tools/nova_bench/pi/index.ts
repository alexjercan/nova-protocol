// The nova_bench relay extension for pi: three tools, each one JSON line to
// the referee's unix socket at NOVA_BENCH_SOCKET and one line back. The
// referee owns the clock and the score; this file owns nothing.
//
// Loaded by `bench play --agent pi` as
//   pi --mode rpc --no-builtin-tools --tools observe,act,finish -e tools/nova_bench/pi/index.ts
// and never meant for an interactive session.

import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { Type } from "typebox";
import * as net from "node:net";

function ask(request: unknown): Promise<string> {
  const path = process.env.NOVA_BENCH_SOCKET;
  if (!path) {
    return Promise.reject(new Error("NOVA_BENCH_SOCKET is not set; run under `bench play --agent pi`"));
  }
  return new Promise((resolve, reject) => {
    const socket = net.connect(path);
    let buffer = "";
    let answered = false;
    socket.setEncoding("utf8");
    socket.on("connect", () => {
      socket.write(JSON.stringify(request) + "\n");
    });
    socket.on("data", (chunk: string) => {
      buffer += chunk;
      const newline = buffer.indexOf("\n");
      if (newline >= 0 && !answered) {
        answered = true;
        resolve(buffer.slice(0, newline));
        socket.end();
      }
    });
    socket.on("error", (error) => {
      if (!answered) {
        answered = true;
        reject(new Error(`the referee is not answering (${error.message}); the run is over`));
      }
    });
    socket.on("close", () => {
      if (!answered) {
        answered = true;
        reject(new Error("the referee closed the connection; the run is over"));
      }
    });
  });
}

// A refused request is a tool ERROR: pi only sets `isError` on a thrown
// error, never on a returned value, so the model sees it flagged and the
// audit records it as one.
function reply(line: string) {
  const parsed = JSON.parse(line);
  if (parsed.error !== undefined) {
    throw new Error(String(parsed.error));
  }
  return {
    content: [{ type: "text" as const, text: JSON.stringify(parsed.ok, null, 1) }],
    details: parsed.ok,
  };
}

const Gesture = Type.Record(Type.String(), Type.Any(), {
  description:
    "One gesture object with one verb: {press: wire} | {release: wire} | {tap: wire} | " +
    "{aim: wire, delta: [x, y], ticks: k} | {command: line}",
});

export default function (pi: ExtensionAPI) {
  pi.registerTool({
    name: "observe",
    label: "Observe",
    description: "The pilot's current view of the game. Free: the clock does not move.",
    parameters: Type.Object({}),
    async execute() {
      return reply(await ask({ observe: {} }));
    },
  });

  pi.registerTool({
    name: "act",
    label: "Act",
    description:
      "Apply gestures, then run the world `ticks` ticks (60 per second, default 30) and " +
      "return the view after. The only way time passes.",
    parameters: Type.Object({
      gestures: Type.Array(Gesture, { description: "Gestures applied on the next tick, in order." }),
      ticks: Type.Optional(
        Type.Integer({ minimum: 0, description: "Ticks to run after the gestures; 60 is one second." }),
      ),
    }),
    async execute(_toolCallId, params) {
      return reply(await ask({ act: { gestures: params.gestures, ticks: params.ticks ?? 30 } }));
    },
  });

  pi.registerTool({
    name: "finish",
    label: "Finish",
    description:
      "End the run: the goal is reached (done) or cannot be reached (gave_up). " +
      "The referee scores from the game state, not from the report.",
    parameters: Type.Object({
      status: Type.Union([Type.Literal("done"), Type.Literal("gave_up")]),
      report: Type.String({ description: "What happened and why, in a few sentences." }),
    }),
    async execute(_toolCallId, params) {
      return reply(await ask({ finish: { status: params.status, report: params.report } }));
    },
  });
}
