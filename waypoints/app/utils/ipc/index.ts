import * as charts from "./charts";
import * as marks from "./marks";
import * as editor from "./editor";
import * as sync from "./sync";
import * as territory from "./territory";

export { listen } from "./invoke";
export type { IPCError, Transport } from "./invoke";

export const ipc = { charts, marks, editor, sync, territory };
