import { createConsola, LogLevels } from "consola";

export function createLogger(tag: string) {
  return createConsola({
    level: import.meta.dev ? LogLevels.debug : LogLevels.info,
  }).withTag(tag);
}
