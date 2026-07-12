export default async () => {
  const ported = document.body.querySelector(".ported-static-paragraph");
  if (!ported) throw new Error("expected teleported content");
  if (ported.innerHTMLString() !== "static teleported") {
    throw new Error("mismatch: " + ported.innerHTMLString());
  }
};
