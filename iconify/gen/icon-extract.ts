import { readFile, writeFile } from "node:fs/promises";
import { iconToSVG, iconToHTML, parseIconSetAsync } from "@iconify/utils";
import { locate } from "@iconify/json";
import nunjucks from "nunjucks";

const prefixes = {
  "tabler": "Tabler",
  "ph": "Phosphor",
};

for (const [prefix, IconSet] of Object.entries(prefixes)) {
  const filename = locate(prefix);

  const iconsetJSON = JSON.parse(await readFile(filename, "utf-8"));

  const icons: Record<string, string> = Object.create(null);
  await parseIconSetAsync(iconsetJSON, async (name, data) => {
    if (!data) return;

    const { attributes, body } = iconToSVG(data, { height: "auto" });
    const svg = iconToHTML(body, attributes);

    icons[name] = svg.replaceAll('"', "'");
  });

  const env = nunjucks.configure({ autoescape: false })
  const json = env.render("./gen/iconset.json.njk", { icons });

  await writeFile(`./icons/${prefix}.json`, json, "utf-8")
}
