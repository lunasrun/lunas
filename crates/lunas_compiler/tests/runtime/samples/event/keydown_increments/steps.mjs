export default async ({ $, dispatch, expect }) => {
  await dispatch("input", "keydown", { key: "a" });
  expect("span").text("1");
};
