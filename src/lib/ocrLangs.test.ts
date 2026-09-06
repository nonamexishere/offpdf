import { describe, expect, it } from "vitest";
import { joinOcrLangs, ocrLangLabel } from "./ocrLangs";

describe("ocrLangLabel", () => {
  it("ocr-lang-label-known: eng/tur/deu/fra → English/Turkish/German/French", () => {
    expect(ocrLangLabel("eng")).toBe("English");
    expect(ocrLangLabel("tur")).toBe("Turkish");
    expect(ocrLangLabel("deu")).toBe("German");
    expect(ocrLangLabel("fra")).toBe("French");
  });

  it("ocr-lang-label-unknown: snum/zzz stay the raw code", () => {
    expect(ocrLangLabel("snum")).toBe("snum");
    expect(ocrLangLabel("zzz")).toBe("zzz");
  });
});

describe("joinOcrLangs", () => {
  it("ocr-join-langs-sorted: sorts by code, strips duplicates, joins with +", () => {
    expect(joinOcrLangs(["tur", "eng"])).toBe("eng+tur");
    expect(joinOcrLangs(["tur", "eng", "tur"])).toBe("eng+tur");
    expect(joinOcrLangs(["fra", "deu"])).toBe("deu+fra");
  });
});
