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

  it("recognizes a 24-word phrase, normalizing case and whitespace", () => {
    const phrase = Array.from({ length: 24 }, () => "abandon").join(" ");
    const messy = `  ${phrase.toUpperCase().replaceAll(" ", "   ")}\n`;
    expect(classifySetupCode(messy)).toEqual({ kind: "phrase", phrase });
  });

  it("recognizes a 12-word phrase", () => {
    const phrase = "legal winner thank year wave sausage worth useful legal winner thank yellow";
    expect(classifySetupCode(phrase)).toEqual({ kind: "phrase", phrase });
  });

  it("rejects word counts that are not a BIP-39 length", () => {
    expect(classifySetupCode("just five words in here")).toEqual({ kind: "unknown" });
    const thirteen = Array.from({ length: 13 }, () => "abandon").join(" ");
    expect(classifySetupCode(thirteen)).toEqual({ kind: "unknown" });
  });

  it("rejects phrase-length input containing non-alphabetic tokens", () => {
    const words = [...Array.from({ length: 11 }, () => "abandon"), "n0tw0rd"];
    expect(classifySetupCode(words.join(" "))).toEqual({ kind: "unknown" });
  });
});
