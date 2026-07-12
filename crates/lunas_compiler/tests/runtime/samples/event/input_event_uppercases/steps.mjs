export default async ({ $, dispatch, expect }) => {
  expect("span").text("");
  await dispatch("input", "input");
  expect("span").text("typed");
};
