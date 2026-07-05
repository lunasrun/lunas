export default async ({ $$, click }) => {
  await click("button");
  const [first, second] = $$("p");
  if (first.getAttribute("data-marked") !== "yes") {
    throw new Error(":ref on a fragment's first root did not resolve correctly");
  }
  if (second.getAttribute("data-marked")) {
    throw new Error("second <p> should be unaffected");
  }
};
