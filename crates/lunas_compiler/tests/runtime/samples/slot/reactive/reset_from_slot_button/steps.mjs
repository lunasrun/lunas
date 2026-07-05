export default async ({ $, $$, click, equal }) => {
  const n = () => $$("li").length;
  equal(n(), 2);
  await click(".add");
  equal(n(), 3);
  await click(".clr");
  equal(n(), 0);
};
