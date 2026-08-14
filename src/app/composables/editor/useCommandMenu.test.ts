import { describe, it, expect } from "vitest";

import { useCommandMenu } from "./useCommandMenu";

describe("useCommandMenu", () => {
  describe("show / hide", () => {
    it("starts closed", () => {
      const menu = useCommandMenu();
      expect(menu.isOpen.value).toBe(false);
      expect(menu.activeMenu.value).toBeNull();
      expect(menu.menuKind.value).toBeNull();
    });

    it("opens with correct state", () => {
      const menu = useCommandMenu();
      menu.show("slash", "b1", "hea", { x: 100, y: 200 });

      expect(menu.isOpen.value).toBe(true);
      expect(menu.menuKind.value).toBe("slash");
      expect(menu.activeMenu.value).toEqual({
        kind: "slash",
        blockID: "b1",
        query: "hea",
        position: { x: 100, y: 200 },
      });
    });

    it("hides and resets state", () => {
      const menu = useCommandMenu();
      menu.show("slash", "b1", "hea", { x: 100, y: 200 });
      menu.hide();

      expect(menu.isOpen.value).toBe(false);
      expect(menu.activeMenu.value).toBeNull();
      expect(menu.menuKind.value).toBeNull();
    });
  });

  describe("updateQuery", () => {
    it("updates query when menu is open", () => {
      const menu = useCommandMenu();
      menu.show("slash", "b1", "", { x: 0, y: 0 });
      menu.updateQuery("heading");

      expect(menu.activeMenu.value!.query).toBe("heading");
    });

    it("preserves other fields", () => {
      const menu = useCommandMenu();
      menu.show("slash", "b1", "old", { x: 10, y: 20 });
      menu.updateQuery("new");

      expect(menu.activeMenu.value!.blockID).toBe("b1");
      expect(menu.activeMenu.value!.kind).toBe("slash");
      expect(menu.activeMenu.value!.position).toEqual({ x: 10, y: 20 });
    });

    it("is a no-op when menu is closed", () => {
      const menu = useCommandMenu();
      menu.updateQuery("test");
      expect(menu.activeMenu.value).toBeNull();
    });
  });

  describe("updatePosition", () => {
    it("updates position when menu is open", () => {
      const menu = useCommandMenu();
      menu.show("slash", "b1", "", { x: 0, y: 0 });
      menu.updatePosition({ x: 50, y: 75 });

      expect(menu.activeMenu.value!.position).toEqual({ x: 50, y: 75 });
    });

    it("is a no-op when menu is closed", () => {
      const menu = useCommandMenu();
      menu.updatePosition({ x: 50, y: 75 });
      expect(menu.activeMenu.value).toBeNull();
    });
  });

  describe("handleKey", () => {
    it("returns false when menu is closed", () => {
      const menu = useCommandMenu();
      const event = new KeyboardEvent("keydown", { key: "ArrowDown" });
      expect(menu.handleKey(event)).toBe(false);
    });

    it("consumes arrow keys when menu is open", () => {
      const menu = useCommandMenu();
      menu.show("slash", "b1", "", { x: 0, y: 0 });

      expect(menu.handleKey(new KeyboardEvent("keydown", { key: "ArrowUp" }))).toBe(true);
      expect(menu.handleKey(new KeyboardEvent("keydown", { key: "ArrowDown" }))).toBe(true);
    });

    it("consumes Enter, Tab, and Escape when menu is open", () => {
      const menu = useCommandMenu();
      menu.show("slash", "b1", "", { x: 0, y: 0 });

      expect(menu.handleKey(new KeyboardEvent("keydown", { key: "Enter" }))).toBe(true);
      expect(menu.handleKey(new KeyboardEvent("keydown", { key: "Tab" }))).toBe(true);
      expect(menu.handleKey(new KeyboardEvent("keydown", { key: "Escape" }))).toBe(true);
    });

    it("does not consume other keys", () => {
      const menu = useCommandMenu();
      menu.show("slash", "b1", "", { x: 0, y: 0 });

      expect(menu.handleKey(new KeyboardEvent("keydown", { key: "a" }))).toBe(false);
      expect(menu.handleKey(new KeyboardEvent("keydown", { key: "Backspace" }))).toBe(false);
    });
  });
});
