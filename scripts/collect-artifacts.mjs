// Reúne en una carpeta plana los archivos que debe subir un job de CI (según la tabla de
// release-targets.mjs) y falla si falta alguno. Renombra el `.app.tar.gz` de macOS.
//   node scripts/collect-artifacts.mjs TARGET BUNDLE_DIR OUT_DIR [--signed]
import { copyFile, mkdir } from "node:fs/promises";
import { join } from "node:path";
import { ARTIFACTS, PRODUCT, listFiles, readVersion, unique } from "./release-targets.mjs";

export async function collect(target, bundleDirectory, outputDirectory, signed, version) {
  const artifacts = ARTIFACTS.filter((a) => a.target === target);
  if (artifacts.length === 0) throw new Error(`Unknown target ${target}`);
  const files = await listFiles(bundleDirectory);
  await mkdir(outputDirectory, { recursive: true });
  const copied = [];
  for (const artifact of artifacts) {
    const wantsSignature = signed && artifact.updater;
    // Sin firma del updater, Tauri no genera los archivos que solo sirven para actualizar.
    if (!artifact.installer && !wantsSignature) continue;
    const built = artifact.built ?? artifact.suffix(version);
    for (const ext of wantsSignature ? ["", ".sig"] : [""]) {
      const file = unique(files, `${built}${ext}`, "built");
      const name = artifact.built ? `${PRODUCT}${artifact.suffix(version)}${ext}` : file.name;
      await copyFile(join(file.parentPath, file.name), join(outputDirectory, name));
      copied.push(name);
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
