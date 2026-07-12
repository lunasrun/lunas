export default async ({ click }) => {
  const ported = () => document.body.querySelector(".ported-ref-toggled");
  if (ported()) throw new Error("should start hidden");
  await click(".toggle");
  if (!ported()) throw new Error("should show after toggle");
  await click(".mark");
  if (ported().getAttribute("data-marked") !== "yes") {
    throw new Error(":ref inside toggled teleport content did not resolve");
  }
};
