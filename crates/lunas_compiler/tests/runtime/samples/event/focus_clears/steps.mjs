export default async ({ $, dispatch, tick, expect }) => {
  expect("input").value("alice");
  await dispatch("input", "focus");
  expect("input").value("");
};
