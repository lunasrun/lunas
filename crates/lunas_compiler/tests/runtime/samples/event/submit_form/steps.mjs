export default async ({ $, dispatch, expect }) => {
  expect("span").text("false");
  await dispatch("form", "submit");
  expect("span").text("true");
};
