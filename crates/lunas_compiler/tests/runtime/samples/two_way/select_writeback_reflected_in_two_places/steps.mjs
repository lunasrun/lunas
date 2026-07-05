export default async ({ setValue, expect }) => {
  expect(".a").text("light");
  expect(".b").text("mode is light");
  await setValue("select", "dark");
  expect(".a").text("dark");
  expect(".b").text("mode is dark");
};
