export default async ({ click, expect }) => {
  expect("span").text("0");
  const btn = document.body.querySelector(".ported-event-handler");
  await click(btn);
  await click(btn);
  expect("span").text("2");
};
