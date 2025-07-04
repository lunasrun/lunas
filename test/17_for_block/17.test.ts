// tests/nested-blocks.spec.ts
import { describe, it, expect } from "vitest";
import NestedBlocks from "./17.lun"; // ★ adjust path
import { pickWithSeed } from "../utils.js";

/**
 * helper — mount component into a fresh container
 */
function mountComponent() {
  const container = document.createElement("div");
  document.body.appendChild(container);
  const instance = NestedBlocks();
  instance.mount(container);
  return { container, instance };
}

describe("NestedBlocks component", () => {
  /* ------------------------------------------------------------------ *
   * 0. initial render
   * ------------------------------------------------------------------ */
  it("should render full 6-level structure with default counts=1", () => {
    const { container } = mountComponent();

    // quick sanity check: deepest block exists
    const deep = container.querySelector("#block-0-0-0-0-0-0")!;
    expect(deep.textContent).toBe("Block: 0-0-0-0-0-0");

    // snapshot style comparison (whitespace-agnostic)
    expect(container.innerHTML).toEqualNormalizedHtml(`
    <div id="root" style="display: flex; gap: 2rem;">
      <div id="controls">
        <h2 id="controls-title">Controls</h2>
        ${[0, 1, 2, 3, 4, 5]
          .map(
            (i) => `<div>
            <button id="toggle-if-${i}">Toggle If${i + 1}: ON</button>
          </div>`,
          )
          .join("\n")}
        ${[0, 1, 2, 3, 4, 5]
          .map(
            (i) => `<div>
            <button id="dec-for-${i}">- For${i + 1}</button>
            <span id="count-for-${i}"> Count: 1 </span>
            <button id="inc-for-${i}">+ For${i + 1}</button>
          </div>`,
          )
          .join("\n")}
      </div>
      <div id="nested">
        <h2 id="nested-title">Nested Blocks</h2>
        <div id="for-1-0">
          <div id="label-for-1-0">FOR 1 (i1=0)</div>
          <div id="if-1-0">
            <div id="label-if-1-0">IF 1 (bools[0] == true)</div>
            <div id="for-2-0-0">
              <div id="label-for-2-0-0">FOR 2 (i2=0)</div>
              <div id="if-2-0-0">
                <div id="label-if-2-0-0">IF 2 (bools[1] == true)</div>
                <div id="for-3-0-0-0">
                  <div id="label-for-3-0-0-0">FOR 3 (i3=0)</div>
                  <div id="if-3-0-0-0">
                    <div id="label-if-3-0-0-0">IF 3 (bools[2] == true)</div>
                    <div id="for-4-0-0-0-0">
                      <div id="label-for-4-0-0-0-0">FOR 4 (i4=0)</div>
                      <div id="if-4-0-0-0-0">
                        <div id="label-if-4-0-0-0-0">IF 4 (bools[3] == true)</div>
                        <div id="for-5-0-0-0-0-0">
                          <div id="label-for-5-0-0-0-0-0">FOR 5 (i5=0)</div>
                          <div id="if-5-0-0-0-0-0">
                            <div id="label-if-5-0-0-0-0-0">IF 5 (bools[4] == true)</div>
                            <div id="for-6-0-0-0-0-0-0">
                              <div id="label-for-6-0-0-0-0-0-0">FOR 6 (i6=0)</div>
                              <div id="if-6-0-0-0-0-0-0">
                                <div id="label-if-6-0-0-0-0-0-0">IF 6 (bools[5] == true)</div>
                                <div id="block-0-0-0-0-0-0">Block: 0-0-0-0-0-0</div>
                              </div>
                            </div>
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  `);
  });

  /* ------------------------------------------------------------------ *
   * 1. toggle bools[0] should hide / show the whole branch
   * ------------------------------------------------------------------ */
  it("should collapse FOR1 branch when bools[0] toggled OFF", async () => {
    const { container } = mountComponent();
    container.querySelector<HTMLButtonElement>("#toggle-if-0")!.click();
    await Promise.resolve();

    expect(container.querySelector("#if-1-0")).toBeNull();
    // ensure deeper levels are removed as well
    expect(container.querySelector("#block-0-0-0-0-0-0")).toBeNull();

    // toggle back ON
    container.querySelector<HTMLButtonElement>("#toggle-if-0")!.click();
    await Promise.resolve();
    expect(container.querySelector("#block-0-0-0-0-0-0")).not.toBeNull();
  });

  /* ------------------------------------------------------------------ *
   * 2. counts[0] ++ should create a second top-level FOR1 node
   * ------------------------------------------------------------------ */
  it("should add a new FOR-1 element when counts[0] is incremented", async () => {
    const { container } = mountComponent();
    container.querySelector<HTMLButtonElement>("#inc-for-0")!.click();
    await Promise.resolve();

    // expect new for-1-1 branch exists
    expect(container.querySelector("#for-1-1")).not.toBeNull();
    // deepest block for the new branch should exist too
    expect(container.querySelector("#block-1-0-0-0-0-0")).not.toBeNull();
  });

  /* ------------------------------------------------------------------ *
   * 3. counts[1] decrement to 0 should clear FOR-2 level under every FOR-1
   * ------------------------------------------------------------------ */
  it("should remove FOR-2 loops when counts[1] decremented to zero", async () => {
    const { container } = mountComponent();
    container.querySelector<HTMLButtonElement>("#dec-for-1")!.click();
    await Promise.resolve();

    // FOR-2 nodes should disappear
    expect(container.querySelector("[id^='for-2-0']")).toBeNull();
    // deepest block must be gone
    expect(container.querySelector("#block-0-0-0-0-0-0")).toBeNull();
  });

  /* ------------------------------------------------------------------ *
   * 4. deep toggle (bools[3]) affects level-4 condition only
   * ------------------------------------------------------------------ */
  it("should hide / restore IF-4 without affecting upper levels", async () => {
    const { container } = mountComponent();

    // console.log(container.innerHTML);

    // sanity: all present
    expect(container.querySelector("#if-4-0-0-0-0")).not.toBeNull();

    // toggle bools[3] OFF
    container.querySelector<HTMLButtonElement>("#toggle-if-3")!.click();
    await Promise.resolve();
    expect(container.querySelector("#if-4-0-0-0-0")).toBeNull();
    // upper IF-3 still exists
    expect(container.querySelector("#if-3-0-0-0")).not.toBeNull();

    // toggle back ON
    container.querySelector<HTMLButtonElement>("#toggle-if-3")!.click();
    await Promise.resolve();
    expect(container.querySelector("#if-4-0-0-0-0")).not.toBeNull();
  });

  it("should handle random button clicks without errors", async () => {
    const { container } = mountComponent();

    const randomButtons = [
      "toggle-if-0",
      "toggle-if-1",
      "toggle-if-2",
      "toggle-if-3",
      "toggle-if-4",
      "toggle-if-5",
      "dec-for-0",
      "count-for-0",
      "inc-for-0",
      "dec-for-1",
      "count-for-1",
      "inc-for-1",
      "dec-for-2",
      "count-for-2",
      "inc-for-2",
      "dec-for-3",
      "count-for-3",
      "inc-for-3",
      "dec-for-4",
      "count-for-4",
      "inc-for-4",
      "dec-for-5",
      "count-for-5",
      "inc-for-5",
    ];

    const pickedRandomButtons = pickWithSeed(randomButtons, 500, "test-seed");

    console.log("Picked random buttons:", pickedRandomButtons);

    // click random buttons
    for (let idx = 0; idx < pickedRandomButtons.length; idx++) {
      const id = pickedRandomButtons[idx];
      const button = container.querySelector<HTMLButtonElement>(`#${id}`);
      if (button) button.click();
      console.log(idx);
      // await Promise.resolve();
      await new Promise((resolve) => setTimeout(resolve, 1));
    }
  });

  /* ------------------------------------------------------------------ *
   * 5. multiple quick operations should batch to a single micro-task
   * ------------------------------------------------------------------ */
  it("should batch updates within the same tick", async () => {
    const { container } = mountComponent();
    // // click 3 buttons synchronously
    const incBtn0 = container.querySelector<HTMLButtonElement>("#inc-for-0")!;
    const incBtn1 = container.querySelector<HTMLButtonElement>("#inc-for-1")!;
    const toggle2 = container.querySelector<HTMLButtonElement>("#toggle-if-2")!;
    incBtn0.click();
    await Promise.resolve();
    incBtn1.click();
    await Promise.resolve();
    toggle2.click();
    await Promise.resolve();

    // updated DOM reflects all actions exactly once
    expect(container.querySelectorAll("#for-1-1").length).toBe(1);
    expect(container.querySelectorAll("#for-2-0-1").length).toBe(1);
    expect(container.querySelector("#if-4-0-0-0-0")).toBeNull();
  });

  /* ------------------------------------------------------------------ *
   * 6. unmount should fully clean the DOM and dependencies
   * ------------------------------------------------------------------ */

  it("should detach everything on __unmount()", () => {
    const { container, instance } = mountComponent();
    instance.__unmount();
    expect(container.innerHTML).toBe("");
  });
});
