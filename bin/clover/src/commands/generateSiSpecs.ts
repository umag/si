import { getServiceByName, loadCfDatabase } from "../cfDb.ts";
import { pkgSpecFromCf } from "../specPipeline.ts";
import { PkgSpec } from "../bindings/PkgSpec.ts";
import { generateAssetFuncs } from "../pipeline-steps/generateAssetFuncs.ts";
import { generateDefaultActionFuncs } from "../pipeline-steps/generateActionFuncs.ts";
import { generateDefaultLeafFuncs } from "../pipeline-steps/generateLeafFuncs.ts";
import { generateDefaultManagementFuncs } from "../pipeline-steps/generateManagementFuncs.ts";
import { addDefaultPropsAndSockets } from "../pipeline-steps/addDefaultPropsAndSockets.ts";
import { generateOutputSocketsFromResourceProps } from "../pipeline-steps/generateSocketsFromResourceProps.ts";
import { generateSubAssets } from "../pipeline-steps/generateSubAssets.ts";
import { generateIntrinsicFuncs } from "../pipeline-steps/generateIntrinsicFuncs.ts";
import { createInputSocketsBasedOnOutputSockets } from "../pipeline-steps/createInputSocketsAcrossAssets.ts";
import { emptyDirectory } from "../util.ts";
import { updateSchemaIdsForExistingSpecs } from "../pipeline-steps/updateSchemaIdsForExistingSpecs.ts";
import { getExistingSpecs } from "../specUpdates.ts";

import _logger from "../logger.ts";

const logger = _logger.ns("siSpecs").seal();
const SI_SPEC_DIR = "si-specs";

export function generateSiSpecForService(serviceName: string) {
  const cf = getServiceByName(serviceName);
  return pkgSpecFromCf(cf);
}

export async function generateSiSpecs() {
  const db = await loadCfDatabase();
  const existing_specs = await getExistingSpecs();

  let imported = 0;
  const cfSchemas = Object.values(db);

  let specs = [] as PkgSpec[];

  for (const cfSchema of cfSchemas) {
    try {
      const pkg = pkgSpecFromCf(cfSchema);

      specs.push(pkg);
    } catch (e) {
      console.log(`Error Building: ${cfSchema.typeName}: ${e}`);
    }
  }

  // EXECUTE PIPELINE STEPS
  specs = generateOutputSocketsFromResourceProps(specs);
  specs = addDefaultPropsAndSockets(specs);
  specs = generateDefaultActionFuncs(specs);
  specs = generateDefaultLeafFuncs(specs);
  specs = generateDefaultManagementFuncs(specs);
  // subAssets should not have any of the above, but need an asset func and
  // intrinsics
  specs = generateSubAssets(specs);
  specs = generateIntrinsicFuncs(specs);
  specs = createInputSocketsBasedOnOutputSockets(specs);
  specs = generateAssetFuncs(specs);
  specs = updateSchemaIdsForExistingSpecs(existing_specs, specs);

  // WRITE OUTS SPECS
  await emptyDirectory(SI_SPEC_DIR);
  for (const spec of specs) {
    const specJson = JSON.stringify(spec, null, 2);
    const name = spec.name;

    try {
      logger.debug(`Writing ${name}.json`);
      await Deno.writeTextFile(
        `${SI_SPEC_DIR}/${name}.json`,
        specJson,
      );
    } catch (e) {
      console.log(`Error writing to file: ${name}: ${e}`);
      continue;
    }

    imported += 1;
  }

  console.log(`built ${imported} out of ${cfSchemas.length}`);
}
