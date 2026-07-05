export default async ({ $$, expect, click }) => {
  expect($$("button")[0]).prop("disabled", true);
  await click($$("button")[1]);
  expect($$("button")[0]).prop("disabled", false);
};
