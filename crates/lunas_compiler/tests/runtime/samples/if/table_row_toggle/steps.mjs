export default async ({ $$, click, expect }) => {
  expect("td.opt").count(0);
  await click($$("button.tog")[0]);
  expect("td.opt").count(1);
  expect("td.opt").text("optional");
  await click($$("button.tog")[0]);
  expect("td.opt").count(0);
};
