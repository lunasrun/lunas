export default async ({ $$, click, expect }) => {
  const [go, disable] = $$("button");
  expect(go).prop("disabled", false);
  await click(disable);
  expect(go).prop("disabled", true);
};
