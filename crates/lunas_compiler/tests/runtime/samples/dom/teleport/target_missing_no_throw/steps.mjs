export default async ({ expect }) => {
  // to="#does-not-exist-anywhere" never resolves: the component still mounts
  // fine (no throw) and the teleported content is simply never inserted
  // anywhere, per built-ins/teleport.md.
  expect("p").text("still renders fine");
  if (document.body.querySelector(".ported-missing-target")) {
    throw new Error("content should not have landed anywhere");
  }
};
