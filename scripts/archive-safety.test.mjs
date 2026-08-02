import assert from "node:assert/strict";
import test from "node:test";
import { validateArchiveEntries } from "./archive-safety.mjs";

test("safe relative archive entries are accepted", () => {
  assert.doesNotThrow(() => validateArchiveEntries(["bin/ffmpeg.exe", "LICENSE.txt"]));
});

for (const unsafe of ["../escape.exe", "folder/../../escape.exe", "/absolute.exe", "C:/drive.exe", "\\\\server\\share\\file.exe"]) {
  test(`unsafe archive entry is rejected: ${unsafe}`, () => {
    assert.throws(() => validateArchiveEntries([unsafe]), /허용되지 않은 경로/);
  });
}
