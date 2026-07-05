export default async ({ $, dispatch, expect }) => {
  expect("span").text("false");
  await dispatch("input", "blur");
  expect("span").text("true");
};
