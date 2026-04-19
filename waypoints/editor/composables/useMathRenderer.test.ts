import { describe, it, expect, vi, beforeEach } from "vitest";

const mockRenderToString = vi.fn((expr: string) => `<span class="katex">${expr}</span>`);

vi.mock("katex", () => ({
  default: {
    renderToString: (...args: any[]) => mockRenderToString(...args),
  },
}));

const mockInvoke = vi.fn(async () => "<svg>typst</svg>");

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: any[]) => mockInvoke(...args),
}));

// Import AFTER mocks are set up
const { useMathRenderer } = await import("./useMathRenderer");

beforeEach(() => {
  mockRenderToString.mockClear();
  mockInvoke.mockClear();
});

describe("useMathRenderer", () => {
  describe("katex rendering", () => {
    it("renders a simple expression", async () => {
      const { render } = useMathRenderer();
      const result = await render("x^2", "katex");

      expect(result.html).toContain("katex");
      expect(mockRenderToString).toHaveBeenCalledWith("x^2", {
        displayMode: false,
        throwOnError: false,
      });
    });

    it("returns cached result on second call", async () => {
      const { render } = useMathRenderer();
      await render("a+b", "katex");
      await render("a+b", "katex");

      // Only called once — second call hits cache
      expect(mockRenderToString).toHaveBeenCalledTimes(1);
    });

    it("rejects empty expressions", async () => {
      const { render } = useMathRenderer();
      const result = await render("", "katex");

      expect(result.html).toBe("");
      expect(result.error).toBeDefined();
      expect(mockRenderToString).not.toHaveBeenCalled();
    });

    it("rejects display-mode LaTeX (\\begin{...})", async () => {
      const { render } = useMathRenderer();
      const result = await render("\\begin{align}x\\end{align}", "katex");

      expect(result.html).toBe("");
      expect(result.error).toBeDefined();
      expect(mockRenderToString).not.toHaveBeenCalled();
    });

    it("strips newlines for inline constraint", async () => {
      const { render } = useMathRenderer();
      await render("x +\ny", "katex");

      expect(mockRenderToString).toHaveBeenCalledWith("x + y", expect.any(Object));
    });

    it("handles katex errors gracefully", async () => {
      mockRenderToString.mockImplementationOnce(() => {
        throw new Error("KaTeX parse error");
      });

      const { render } = useMathRenderer();
      const result = await render("\\invalid", "katex");

      expect(result.html).toBe("");
      expect(result.error).toContain("KaTeX parse error");
    });
  });

  describe("typst rendering", () => {
    it("calls Tauri IPC with correct args", async () => {
      const { render } = useMathRenderer();
      await render("x^2", "typst");

      expect(mockInvoke).toHaveBeenCalledWith("editor_render_math", {
        expression: "x^2",
        engine: "typst",
        displayMode: false,
      });
    });

    it("passes displayMode flag", async () => {
      const { render } = useMathRenderer();
      await render("x^2", "typst", true);

      expect(mockInvoke).toHaveBeenCalledWith("editor_render_math", {
        expression: "x^2",
        engine: "typst",
        displayMode: true,
      });
    });

    it("falls back to monospace code on IPC failure", async () => {
      mockInvoke.mockRejectedValueOnce(new Error("IPC unavailable"));

      const { render } = useMathRenderer();
      // Use unique expression to avoid cache hits from other tests
      const result = await render("fallback_expr_1", "typst");

      expect(result.html).toContain("<code");
      expect(result.html).toContain("fallback_expr_1");
      expect(result.error).toContain("IPC unavailable");
    });

    it("escapes HTML in fallback to prevent XSS", async () => {
      mockInvoke.mockRejectedValueOnce(new Error("fail"));

      const { render } = useMathRenderer();
      const result = await render("<script>alert(1)</script>", "typst");

      expect(result.html).not.toContain("<script>");
      expect(result.html).toContain("&lt;script&gt;");
    });

    it("allows display-mode expressions for typst", async () => {
      mockInvoke.mockResolvedValueOnce("<svg>typst-display</svg>");

      const { render } = useMathRenderer();
      // Use unique expression to avoid cache hits
      const result = await render("sum_(k=0)^n k", "typst", true);

      expect(mockInvoke).toHaveBeenCalled();
      expect(result.html).toBe("<svg>typst-display</svg>");
    });
  });

  describe("loading state", () => {
    it("sets loading during render", async () => {
      const { render, loading } = useMathRenderer();
      expect(loading.value).toBe(false);

      const promise = render("x^2", "katex");
      // loading is set synchronously at the start
      expect(loading.value).toBe(true);

      await promise;
      expect(loading.value).toBe(false);
    });

    it("clears loading on error", async () => {
      mockRenderToString.mockImplementationOnce(() => {
        throw new Error("fail");
      });

      const { render, loading } = useMathRenderer();
      await render("bad", "katex");

      expect(loading.value).toBe(false);
    });
  });

  describe("defaults", () => {
    it("defaults to katex engine", async () => {
      const { render } = useMathRenderer();
      // Unique expression to avoid cache hits
      await render("default_engine_test");

      expect(mockRenderToString).toHaveBeenCalled();
      expect(mockInvoke).not.toHaveBeenCalled();
    });
  });
});
