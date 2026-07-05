export default async ({ dispatch, expect }) => {
  expect(".box").text("false");
  await dispatch(".box", "auxclick");
  expect(".box").text("true");
};
