export default async ({ dispatch, expect }) => {
  expect(".box").text("false");
  await dispatch(".box", "pointerdown");
  expect(".box").text("true");
};
