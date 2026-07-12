export default async ({ $$, click, expect }) => {
  const [toA, toB] = $$("button");
  const bumpBtn = () => $$("button")[2];
  expect("span").text("0");
  await click(bumpBtn());
  await click(bumpBtn());
  expect("span").text("2");
  await click(toB);
  expect("p").text("Page B");
  expect("span").count(0);
  await click(toA);
  // `n` is component-level state, so it survived the cascade switch even
  // though the branch's DOM (the <div>/<button>/<span>) was torn down and
  // rebuilt fresh.
  expect("span").text("2");
};
