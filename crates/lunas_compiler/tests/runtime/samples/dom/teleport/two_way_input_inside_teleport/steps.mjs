export default async ({ expect }) => {
  const input = document.body.querySelector(".ported-two-way-input");
  if (!input) throw new Error("expected teleported input");
  if (input.value !== "a") throw new Error("expected initial value 'a'");
  expect("span").text("a");
};
