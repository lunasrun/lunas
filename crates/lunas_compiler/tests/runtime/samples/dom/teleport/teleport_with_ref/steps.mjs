export default async ({ click }) => {
  await click("button");
  const el = document.body.querySelector(".ported-with-ref");
  if (!el) throw new Error("expected teleported node");
  if (el.getAttribute("data-marked") !== "yes") {
    throw new Error(":ref inside teleport content did not resolve to the teleported node");
  }
};
