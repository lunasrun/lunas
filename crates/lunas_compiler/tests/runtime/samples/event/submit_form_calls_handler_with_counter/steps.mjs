export default async ({ $, dispatch, expect }) => {
  await dispatch("form", "submit");
  await dispatch("form", "submit");
  expect("span").text("submits=2");
};
