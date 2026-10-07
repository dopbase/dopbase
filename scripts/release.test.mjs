import assert from "node:assert/strict";
import {
  mkdtempSync,
  mkdirSync,
  writeFileSync,
  readFileSync,
  cpSync,
  rmSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

const scripts = resolve("scripts/release");
function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), "dopbase-release-test-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "bin"));
  mkdirSync(join(root, "assets"));
  for (const name of ["CHANGELOG.md", "LICENSE", "NOTICE"])
    writeFileSync(join(root, name), "fixture\n");
  executable(join(root, "dopbase"), '#!/bin/sh\nprintf "v1.2.3\\n"\n');
  return root;
}
function executable(path, content) {
  writeFileSync(path, content, { mode: 0o755 });
}
function run(root, script, args = [], overrides = {}) {
  return spawnSync("bash", [join(scripts, script), ...args], {
    cwd: root,
    env: {
      ...process.env,
      PATH: `${root}/bin:${process.env.PATH}`,
      ...overrides,
    },
    encoding: "utf8",
  });
}
function pack(root, os = "darwin", arch = "arm64") {
  const result = run(root, "package-archive.sh", [
    join(root, "dopbase"),
    "1.2.3",
    os,
    arch,
    join(root, "assets"),
  ]);
  assert.equal(result.status, 0, result.stderr);
  return join(root, "assets", `dopbase_1.2.3_${os}_${arch}.tar.gz`);
}
function bundle(root) {
  for (const os of ["darwin", "linux"])
    for (const arch of ["amd64", "arm64"]) pack(root, os, arch);
  const result = run(root, "verify-assets.sh", ["1.2.3", join(root, "assets")]);
  assert.equal(result.status, 0, result.stderr);
  writeFileSync(join(root, "assets/release-notes.md"), "Release notes\n");
}
function publisher(root, overrides = {}) {
  mkdirSync(join(root, "remote"));
  executable(
    join(root, "bin/curl"),
    `#!/bin/sh
while [ "$#" -gt 0 ]; do
  case "$1" in --output) output=$2; shift 2 ;; *) shift ;; esac
done
printf '{"id":123,"draft":%s}\n' "\${TEST_DRAFT:-true}" > "$output"
printf '%s' "\${TEST_STATUS:-200}"
`,
  );
  executable(join(root, "bin/sleep"), "#!/bin/sh\nexit 0\n");
  executable(
    join(root, "bin/gh"),
    `#!/bin/sh
printf '%s\n' "$*" >> "$TEST_ROOT/commands"
[ "$1" != api ] || exit 0
operation=$2
shift 3
case "$operation" in
  upload)
    [ "\${TEST_UPLOAD_FAIL:-0}" != 1 ] || exit 1
    cp "$1" "$TEST_ROOT/remote/"
    if [ "\${TEST_CORRUPT_UPLOAD:-0}" = 1 ] && [ "\${1##*/}" != checksums.txt ]; then printf corrupt >> "$TEST_ROOT/remote/\${1##*/}"; fi
    ;;
  download)
    while [ "$#" -gt 0 ]; do
      case "$1" in --pattern) name=$2; shift 2 ;; --dir) directory=$2; shift 2 ;; *) shift ;; esac
    done
    cp "$TEST_ROOT/remote/$name" "$directory/$name"
    ;;
  create|edit) ;;
  *) exit 1 ;;
esac
`,
  );
  if (overrides.TEST_DRAFT === "false")
    cpSync(join(root, "assets"), join(root, "remote"), { recursive: true });
  if (overrides.TEST_REMOTE_MISMATCH === "1")
    writeFileSync(join(root, "remote/checksums.txt"), "invalid\n");
  return run(root, "publish.sh", ["1.2.3", join(root, "assets")], {
    GH_REPO: "example/repo",
    GH_TOKEN: "test-token",
    TEST_ROOT: root,
    ...overrides,
  });
}

test("packaging preserves required files, executable mode, and reproducible metadata", (t) => {
  const root = fixture(t);
  const archive = pack(root);
  const first = readFileSync(archive);
  assert.deepEqual(readFileSync(pack(root)), first);
  const listing = spawnSync("tar", ["-tzf", archive], { encoding: "utf8" });
  assert.equal(listing.stdout, "dopbase\nCHANGELOG.md\nLICENSE\nNOTICE\n");
  assert.equal(first.readUInt32LE(4), 0, "gzip timestamp");
  executable(join(root, "dopbase"), '#!/bin/sh\nprintf "v9.9.9\\n"\n');
  const mismatch = run(root, "package-archive.sh", [
    join(root, "dopbase"),
    "1.2.3",
    "darwin",
    "arm64",
    join(root, "assets"),
  ]);
  assert.notEqual(mismatch.status, 0);
  assert.match(mismatch.stderr, /version does not match/);
});

test("source validation checks tag, both package versions, and dated changelog", (t) => {
  const root = fixture(t);
  writeFileSync(join(root, "package.json"), '{"version":"1.2.3"}');
  writeFileSync(join(root, "CHANGELOG.md"), "## 1.2.3 - 2026-10-07\n");
  executable(
    join(root, "bin/cargo"),
    '#!/bin/sh\nprintf \'{"packages":[{"name":"app","version":"%s"}]}\\n\' "${TEST_APP_VERSION:-1.2.3}"\n',
  );
  assert.equal(run(root, "validate-source.sh", ["1.2.3"]).status, 0);
  for (const version of ["v1.2.3", "1.2", "1.2.3evil"])
    assert.notEqual(run(root, "validate-source.sh", [version]).status, 0);
  assert.notEqual(
    run(root, "validate-source.sh", ["1.2.3"], { TEST_APP_VERSION: "1.2.4" })
      .status,
    0,
  );
  writeFileSync(join(root, "package.json"), '{"version":"1.2.4"}');
  assert.notEqual(run(root, "validate-source.sh", ["1.2.3"]).status, 0);
  writeFileSync(join(root, "package.json"), '{"version":"1.2.3"}');
  writeFileSync(join(root, "CHANGELOG.md"), "## 1.2.3 - Unreleased\n");
  assert.notEqual(run(root, "validate-source.sh", ["1.2.3"]).status, 0);
});

test("asset verification rejects missing, extra, damaged, and unexpected archives", (t) => {
  const root = fixture(t);
  bundle(root);
  const archive = join(root, "assets/dopbase_1.2.3_linux_arm64.tar.gz");
  const saved = readFileSync(archive);
  rmSync(archive);
  assert.notEqual(
    run(root, "verify-assets.sh", ["1.2.3", join(root, "assets")]).status,
    0,
  );
  writeFileSync(archive, saved);
  writeFileSync(join(root, "assets/dopbase_extra.zip"), "extra");
  assert.notEqual(
    run(root, "verify-assets.sh", ["1.2.3", join(root, "assets")]).status,
    0,
  );
  rmSync(join(root, "assets/dopbase_extra.zip"));
  writeFileSync(archive, "corrupt");
  assert.notEqual(
    run(root, "verify-assets.sh", ["1.2.3", join(root, "assets")]).status,
    0,
  );
  const incomplete = spawnSync("tar", ["-czf", archive, "-C", root, "LICENSE"]);
  assert.equal(incomplete.status, 0);
  assert.notEqual(
    run(root, "verify-assets.sh", ["1.2.3", join(root, "assets")]).status,
    0,
  );
});

for (const status of ["404", "200"])
  test(`publisher ${status === "404" ? "creates" : "resumes"} a draft and verifies uploads before publishing`, (t) => {
    const root = fixture(t);
    bundle(root);
    const result = publisher(root, { TEST_STATUS: status });
    assert.equal(result.status, 0, result.stderr);
    const commands = readFileSync(join(root, "commands"), "utf8");
    assert.equal(commands.includes("release create"), status === "404");
    assert.equal((commands.match(/release upload/g) ?? []).length, 5);
    assert.ok(
      commands.lastIndexOf("release download") <
        commands.indexOf("--draft=false"),
    );
    assert.match(commands, /--notes-file/);
  });

test("publisher preserves matching and mismatched published releases", (t) => {
  for (const corrupt of [false, true]) {
    const root = fixture(t);
    bundle(root);
    const result = publisher(root, {
      TEST_DRAFT: "false",
      TEST_REMOTE_MISMATCH: corrupt ? "1" : "0",
    });
    assert.equal(result.status === 0, !corrupt, result.stderr);
    const commands = readFileSync(join(root, "commands"), "utf8");
    assert.doesNotMatch(commands, /release (edit|create|upload)/);
  }
});

test("API, upload, and verification failures never publish a draft", (t) => {
  for (const overrides of [
    { TEST_STATUS: "401" },
    { TEST_STATUS: "500" },
    { TEST_UPLOAD_FAIL: "1" },
    { TEST_CORRUPT_UPLOAD: "1" },
  ]) {
    const root = fixture(t);
    bundle(root);
    const result = publisher(root, overrides);
    assert.notEqual(result.status, 0);
    const commands = readFileSync(join(root, "commands"), "utf8");
    assert.doesNotMatch(commands, /--draft=false/);
    if (overrides.TEST_UPLOAD_FAIL)
      assert.equal((commands.match(/release upload/g) ?? []).length, 3);
    if (overrides.TEST_STATUS)
      assert.doesNotMatch(commands, /release (create|upload|edit)/);
  }
});
