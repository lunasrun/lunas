export default async ({ $$, click, equal, expect }) => {
  const first = () => $$("li")[0].innerHTMLString();
  expect("li").count(50);
  const lastNode = $$("li")[49];
  await click(".go");
  equal(first(), "49");
  equal($$("li")[0] === lastNode, true); // moved node reused, not rebuilt
  expect("li").count(50);
};
