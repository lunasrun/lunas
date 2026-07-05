export default async ({ dispatch, expect }) => {
  expect("span").text("none");
  await dispatch("input", "keydown", { key: "Enter" });
  expect("span").text("x");
};
