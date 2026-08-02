import { isAbsolute, normalize, sep } from "node:path";

export function validateArchiveEntries(entries) {
  for (const entry of entries) {
    const normalized = normalize(entry.replaceAll("\\", "/"));
    if (
      isAbsolute(entry)
      || entry.startsWith("//")
      || entry.startsWith("\\\\")
      || /^[A-Za-z]:/.test(entry)
      || normalized === ".."
      || normalized.startsWith(`..${sep}`)
    ) {
      throw new Error(`압축 파일에 허용되지 않은 경로가 있습니다: ${entry}`);
    }
  }
}
