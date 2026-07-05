export default async ({ click, expect }) => {
  // The teleport target is an Element ref (`:to="targetEl"`), not a global
  // selector -- exclusively this case's own regardless of the shared shim
  // `document`. Target resolution is one-shot at construction (see
  // built-ins/teleport.md), so the teleport is gated behind `:if="ready"`
  // (starts false) to make sure `targetEl` is already assigned by the time
  // the teleport block is actually built.
  expect(".own-target").html("");
  await click("button");
  expect(".own-target").html("<p class=\"ported-element-target\">landed here</p>");
};
