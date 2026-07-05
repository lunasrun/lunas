export default async ({ click }) => {
  if (!document.body.querySelector(".ported-is-info")) {
    throw new Error("expected Info teleported initially");
  }
  await click("button");
  if (document.body.querySelector(".ported-is-info")) {
    throw new Error("Info should be unmounted after swap");
  }
  if (!document.body.querySelector(".ported-is-warning")) {
    throw new Error("expected Warning teleported after swap");
  }
};
