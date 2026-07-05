export default async ({ $, dispatch, expect }) => {
  expect("span").text("0");
  await dispatch("input", "change");
  expect("span").text("1");
};
