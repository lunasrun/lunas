export default async ({ expect }) => {
  expect(".text-item").text("hello");
  expect(".number-item").text("42");
};
