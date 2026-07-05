export default async ({ $, dispatch, expect }) => {
  await dispatch("input", "keydown", { key: "x" });
  expect("span").text("d=1 u=0");
  await dispatch("input", "keyup", { key: "x" });
  expect("span").text("d=1 u=1");
};
