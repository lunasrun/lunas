export default async ({ click }) => {
  // See basic_to_portal/steps.mjs for why we query the shared document by a
  // case-unique class rather than assuming exclusive portal ownership.
  const hasPorted = () => !!document.body.querySelector(".ported-toggled-by-if");
  if (hasPorted()) throw new Error("should start hidden");
  await click("button");
  if (!hasPorted()) throw new Error("should show after toggle");
  await click("button");
  if (hasPorted()) throw new Error("should hide again");
};
