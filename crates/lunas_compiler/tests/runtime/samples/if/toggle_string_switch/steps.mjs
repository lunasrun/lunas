export default async ({ $$, click, expect }) => {
  const [en, fr, jp] = $$("button");
  expect("p").text("Hello");
  await click(fr);
  expect("p").text("Bonjour");
  await click(jp);
  expect("p").text("Konnichiwa");
  await click(en);
  expect("p").text("Hello");
};
