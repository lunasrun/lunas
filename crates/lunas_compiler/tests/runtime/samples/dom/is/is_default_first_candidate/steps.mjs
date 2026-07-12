export default async ({ expect }) => {
  expect(".first").count(1);
  expect(".second").count(0);
};
