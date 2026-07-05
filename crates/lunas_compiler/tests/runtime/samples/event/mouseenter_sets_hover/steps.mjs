export default async ({ $, dispatch, expect }) => {
  expect(".box").text("false");
  await dispatch(".box", "mouseenter");
  expect(".box").text("true");
  await dispatch(".box", "mouseleave");
  expect(".box").text("false");
};
