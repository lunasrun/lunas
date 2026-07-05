export default async ({ $, dispatch, expect }) => {
  expect("span").text("0");
  await dispatch("select", "change");
  expect("span").text("1");
};
