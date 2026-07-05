export default async ({ $, dispatch, expect }) => {
  await dispatch(".area", "mousemove");
  await dispatch(".area", "mousemove");
  await dispatch(".area", "mousemove");
  expect(".area").text("3");
};
