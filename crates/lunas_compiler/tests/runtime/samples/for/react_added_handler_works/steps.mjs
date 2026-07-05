export default async ({ $$, click, equal, expect }) => {
  await click(".add");
  expect("button.p").count(2);
  // newly-added item's handler fires without error
  await click($$("button.p")[1]);
};
