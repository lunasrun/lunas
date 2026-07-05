export default async ({ dispatch, expect }) => {
  await dispatch(".box", "wheel");
  await dispatch(".box", "wheel");
  expect(".box").text("2");
};
