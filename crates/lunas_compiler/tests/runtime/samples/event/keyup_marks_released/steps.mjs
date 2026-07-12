export default async ({ $, dispatch, expect }) => {
  expect("span").text("false");
  await dispatch("input", "keyup", { key: "Enter" });
  expect("span").text("true");
};
