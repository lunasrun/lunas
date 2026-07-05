export default async ({ click, expect }) => {
  expect("span").text("idle");
  const btn = document.body.querySelector(".ported-btn-click");
  if (!btn) throw new Error("expected teleported button");
  await click(btn);
  expect("span").text("marked");
};
