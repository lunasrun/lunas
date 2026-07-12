export default async ({ $, dispatch, expect }) => {
  await dispatch("input", "input");
  await dispatch("input", "input");
  expect("span").text("2");
};
