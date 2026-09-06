import { describe, expect, it } from "vitest";
import { joinOcrLangs, ocrLangLabel, pickSearchOcrLang } from "./ocrLangs";

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

describe("pickSearchOcrLang", () => {
  it("R-SEARCH-LANG-BREW: brew-only installed → eng, not eng+tur", () => {
    expect(pickSearchOcrLang(["eng", "osd", "snum"])).toBe("eng");
  });

  it("R-SEARCH-LANG-BOTH: eng and tur installed → eng+tur", () => {
    expect(pickSearchOcrLang(["eng", "osd", "tur"])).toBe("eng+tur");
  });

  it("R-SEARCH-LANG-EMPTY: no preferred pack → empty string", () => {
    expect(pickSearchOcrLang(["osd", "snum"])).toBe("");
  });

  it("R-SEARCH-LANG-TUR: tur without eng → tur, do not invent eng", () => {
    expect(pickSearchOcrLang(["tur", "fra"])).toBe("tur");
  });
});
