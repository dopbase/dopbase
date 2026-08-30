type ChildName = "Admin UI" | "Rust API";

type Child = {
  name: ChildName;
  process: Bun.Subprocess;
};

const children: Child[] = [
  {
    name: "Admin UI",
    process: Bun.spawn(["bun", "run", "dev:ui"], {
      stdin: "inherit",
      stdout: "inherit",
      stderr: "inherit",
    }),
  },
  {
    name: "Rust API",
    process: Bun.spawn(["bun", "run", "dev:app"], {
      stdin: "inherit",
      stdout: "inherit",
      stderr: "inherit",
    }),
  },
];

console.log("Dopbase development servers starting:");
console.log("  Admin UI: http://localhost:8080");
console.log("  API:      http://localhost:8840/api/v1");
console.log("  Swagger:  http://localhost:8840/api/docs/");

let stopping = false;

async function stop(signal: NodeJS.Signals, exitCode: number): Promise<never> {
  if (!stopping) {
    stopping = true;
    for (const child of children) {
      if (child.process.exitCode === null) {
        child.process.kill(signal);
      }
    }
    await Promise.all(children.map((child) => child.process.exited));
  }
  process.exit(exitCode);
}

process.on("SIGINT", () => void stop("SIGINT", 0));
process.on("SIGTERM", () => void stop("SIGTERM", 0));

const firstExit = await Promise.race(
  children.map(async (child) => ({
    name: child.name,
    exitCode: await child.process.exited,
  })),
);

if (!stopping) {
  console.error(
    `${firstExit.name} exited with code ${firstExit.exitCode}; stopping development.`,
  );
  await stop("SIGTERM", firstExit.exitCode);
}
