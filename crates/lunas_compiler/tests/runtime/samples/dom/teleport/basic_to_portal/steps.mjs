export default async () => {
  // IMPORTANT harness caveat (see README for the general kit; this note is
  // specific to teleport): every sample case in the whole suite shares ONE
  // global shim `document` (see runtime/harness/run-samples.mjs). The runtime's
  // `teleportBlock` resolves a string target with `document.querySelector`,
  // which always returns the FIRST `#portal` div ever created across the
  // *entire* batched run -- not one scoped to this case. `to="body"` cannot be
  // asserted at all: the shim's `querySelector` only searches descendants of
  // `body`, so it can never match `body` itself, and the runtime's "target not
  // found -> silently skip" path (documented in built-ins/teleport.md) applies.
  //
  // So instead of asserting exclusive ownership of the portal, every dom/
  // teleport/* case gives its ported node a case-unique class name and only
  // asserts that ITS OWN marked node appears/disappears as expected --
  // tolerant of other cases' content also living in the same shared portal.
  const ported = document.body.querySelector(".ported-basic-to-portal");
  if (!ported) throw new Error("expected teleported content in the portal");
  if (ported.innerHTMLString() !== "teleported content") {
    throw new Error("teleported content mismatch: " + ported.innerHTMLString());
  }
};
