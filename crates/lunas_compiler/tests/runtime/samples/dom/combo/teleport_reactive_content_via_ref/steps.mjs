export default async ({ click }) => {
  const ported = () => document.body.querySelector(".ported-reactive-ref");
  if (ported().innerHTMLString() !== "hi") {
    throw new Error("expected initial teleported text 'hi'");
  }
  await click("button");
  if (ported().innerHTMLString() !== "bye") {
    throw new Error("expected teleported text to update to 'bye'");
  }
  if (ported().getAttribute("data-marked") !== "yes") {
    throw new Error(":ref inside teleport content did not resolve to the live teleported node");
  }
};
