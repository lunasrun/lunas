export default async ({ $$, expect, click }) => {
  expect($$("button")[0]).prop("disabled", false);
  expect($$("button")[1]).prop("disabled", false);
  await click($$("button")[0]);
  expect($$("button")[0]).prop("disabled", true);
  expect($$("button")[1]).prop("disabled", true);
};
