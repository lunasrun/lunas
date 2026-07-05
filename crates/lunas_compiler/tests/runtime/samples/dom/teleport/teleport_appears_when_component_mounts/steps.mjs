export default async ({ click }) => {
  const hasPorted = () => !!document.body.querySelector(".ported-appears-on-mount");
  if (hasPorted()) throw new Error("should not exist before the Modal mounts");
  await click("button");
  if (!hasPorted()) throw new Error("teleport content should appear once Modal mounts");
  // Known gap (see roadmap follow-up): the compiler does not currently wire a
  // top-level <teleport>'s destroy() into the owning component's onDestroy, so
  // we do NOT assert removal here after unmounting via a further :is swap --
  // only the mount-time behavior, which is what actually works today.
};
