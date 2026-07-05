export default async ({ $, dispatch, expect }) => {
  await dispatch(".btn", "mousedown");
  expect(".btn").text("true");
  await dispatch(".btn", "mouseup");
  expect(".btn").text("false");
};
