// Reúne en una carpeta plana los archivos que debe subir un job de CI (según la tabla de
// release-targets.mjs) y falla si falta alguno.
//   node scripts/collect-artifacts.mjs TARGET BUNDLE_DIR OUT_DIR [--signed]
import { copyFile, mkdir } from "node:fs/promises";
import { join } from "node:path";
import { ARTIFACTS, listFiles, readVersion, unique } from "./release-targets.mjs";

export async function collect(target, bundleDirectory, outputDirectory, signed, version) {
  const artifacts = ARTIFACTS.filter((a) => a.target === target);
  if (artifacts.length === 0) throw new Error(`Unknown target ${target}`);
  const files = await listFiles(bundleDirectory);
  await mkdir(outputDirectory, { recursive: true });
  const copied = [];
  for (const artifact of artifacts) {
    const wantsSignature = signed && artifact.updater;
    for (const ext of wantsSignature ? ["", ".sig"] : [""]) {
      const file = unique(files, `${artifact.suffix(version)}${ext}`, "built");
      await copyFile(join(file.parentPath, file.name), join(outputDirectory, file.name));
      copied.push(file.name);
    }
  }
  return copied;
}

if (import.meta.main) {
  try {
    const [target, bundle, out, flag] = process.argv.slice(2);
    if (!target || !bundle || !out || (flag && flag !== "--signed")) {
      throw new Error("Usage: node scripts/collect-artifacts.mjs TARGET BUNDLE_DIR OUT_DIR [--signed]");
    }
    for (const name of await collect(target, bundle, out, flag === "--signed", await readVersion())) {
      console.log(name);
    }
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
