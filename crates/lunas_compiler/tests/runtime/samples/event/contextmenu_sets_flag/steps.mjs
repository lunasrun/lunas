export default async ({ dispatch, expect }) => {
  expect(".box").text("false");
  await dispatch(".box", "contextmenu");
  expect(".box").text("true");
};
