export default async ({ $, dispatch, expect }) => {
  expect("span").text("idle");
  await dispatch("input", "focus");
  expect("span").text("focused");
  await dispatch("input", "blur");
  expect("span").text("blurred");
};
