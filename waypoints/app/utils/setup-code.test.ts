import { describe, expect, it } from "vitest";
import { classifySetupCode } from "./setup-code";

describe("classifySetupCode", () => {
  it("recognizes raw pairing JSON", () => {
    const code = '{"v":1,"kind":"waypoints-pairing","mnemonic":"a b c"}';
    expect(classifySetupCode(code)).toEqual({ kind: "pairing", code });
  });

  it("trims surrounding whitespace before classifying", () => {
    const code = '{"v":1}';
    expect(classifySetupCode(`  ${code}\n`)).toEqual({ kind: "pairing", code });
  });

  it("rejects empty input", () => {
    expect(classifySetupCode("   \n ")).toEqual({ kind: "unknown" });
  });

  it("rejects arbitrary prose", () => {
    expect(classifySetupCode("hello there, general kenobi")).toEqual({ kind: "unknown" });
  });
});
